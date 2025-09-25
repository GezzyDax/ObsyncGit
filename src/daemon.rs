use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, unbounded};
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::git::GitFacade;
use crate::ignore::IgnoreMatcher;

#[derive(Debug)]
enum SyncEvent {
    Changed,
    Rescan,
    WatcherError(String),
}

pub struct SyncDaemon {
    config: Config,
    git: GitFacade,
    ignore: IgnoreMatcher,
    shutdown: Arc<AtomicBool>,
}

impl SyncDaemon {
    pub fn new(config: Config) -> Result<Self> {
        let git = GitFacade::new(&config)?;
        let ignore = IgnoreMatcher::new(config.workdir.as_std_path(), &config.ignore.globs)?;
        Ok(Self {
            config,
            git,
            ignore,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn run(mut self) -> Result<()> {
        info!(path = %self.config.workdir, "starting ObsyncGit daemon");

        let shutdown = self.shutdown.clone();
        ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
        })
        .context("failed to install Ctrl-C handler")?;

        self.git.ensure_repo(&self.config.repo_url)?;

        if self.config.self_update.enabled {
            info!("self-update is enabled (custom command execution happens via configuration)");
        }

        let (tx, rx) = unbounded();
        let ignore = Arc::new(self.ignore.clone());
        let watcher_shutdown = self.shutdown.clone();
        let debounce = self.config.debounce_duration();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if watcher_shutdown.load(Ordering::SeqCst) {
                    return;
                }
                match res {
                    Ok(event) => {
                        let mut relevant = false;
                        for path in &event.paths {
                            if ignore.should_ignore(path) {
                                continue;
                            }
                            relevant = true;
                        }
                        if relevant {
                            let _ = tx.send(SyncEvent::Changed);
                        }
                        if event.need_rescan() {
                            let _ = tx.send(SyncEvent::Rescan);
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(SyncEvent::WatcherError(err.to_string()));
                    }
                }
            },
            NotifyConfig::default().with_poll_interval(debounce),
        )?;

        watcher
            .watch(self.config.workdir.as_std_path(), RecursiveMode::Recursive)
            .with_context(|| {
                format!(
                    "failed to start filesystem watcher on {}",
                    self.config.workdir
                )
            })?;

        self.event_loop(rx)
    }

    fn event_loop(&mut self, rx: Receiver<SyncEvent>) -> Result<()> {
        let debounce = self.config.debounce_duration();
        let poll_interval = self.config.poll_interval();
        let mut dirty_since: Option<Instant> = None;
        let mut last_poll = Instant::now()
            .checked_sub(poll_interval)
            .unwrap_or_else(Instant::now);
        let mut backoff_until: Option<Instant> = None;
        let mut backoff_step: u32 = 0;

        while !self.shutdown.load(Ordering::SeqCst) {
            let now = Instant::now();

            if let Some(until) = backoff_until
                && now >= until
            {
                backoff_until = None;
                debug!("backoff window elapsed, resuming operations");
            }

            if backoff_until.is_none() {
                if let Some(dirty_at) = dirty_since
                    && now.duration_since(dirty_at) >= debounce
                {
                    match self.sync_once() {
                        Ok(changed) => {
                            if changed {
                                info!("local changes synchronized");
                            }
                            dirty_since = None;
                            backoff_step = 0;
                            last_poll = Instant::now();
                            continue;
                        }
                        Err(err) => {
                            error!(?err, "synchronization failed");
                            backoff_step = (backoff_step + 1).min(6);
                            let backoff = backoff_delay(backoff_step);
                            backoff_until = Some(Instant::now() + backoff);
                            continue;
                        }
                    }
                }

                if now.duration_since(last_poll) >= poll_interval {
                    match self.pull_remote() {
                        Ok(()) => {
                            last_poll = Instant::now();
                            backoff_step = 0;
                        }
                        Err(err) => {
                            warn!(?err, "failed to pull remote updates");
                            backoff_step = (backoff_step + 1).min(6);
                            let backoff = backoff_delay(backoff_step);
                            backoff_until = Some(Instant::now() + backoff);
                        }
                    }
                    continue;
                }
            }

            let timeout = compute_timeout(
                now,
                dirty_since,
                debounce,
                last_poll,
                poll_interval,
                backoff_until,
            );

            match rx.recv_timeout(timeout) {
                Ok(event) => match event {
                    SyncEvent::Changed | SyncEvent::Rescan => {
                        dirty_since = Some(Instant::now());
                        debug!("filesystem change detected");
                    }
                    SyncEvent::WatcherError(msg) => {
                        warn!("watcher error: {msg}");
                    }
                },
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // loop recomputes state
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    warn!("watcher channel disconnected, shutting down");
                    break;
                }
            }
        }

        info!("ObsyncGit shutting down");
        Ok(())
    }

    fn sync_once(&mut self) -> Result<bool> {
        self.git.stage_all()?;
        let files = self.git.list_changed_files()?;
        if files.is_empty() {
            debug!("no staged changes detected");
            return Ok(false);
        }
        let message = self.build_commit_message(&files);
        self.git.commit(&message)?;
        self.git.pull_rebase()?;
        self.git.push()?;
        info!(?files, "pushed commit");
        Ok(true)
    }

    fn pull_remote(&self) -> Result<()> {
        self.git.pull_rebase()?;
        Ok(())
    }

    fn build_commit_message(&self, files: &[String]) -> String {
        use chrono::{SecondsFormat, Utc};

        let cfg = &self.config.commit;
        let prefix = cfg.prefix.trim();
        let summary = if files.len() <= cfg.max_files_in_summary {
            files.join(", ")
        } else {
            format!("updated {} files", files.len())
        };
        let mut message = format!("{} {}", prefix, summary);
        if cfg.include_timestamp {
            let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
            message.push_str(&format!(" ({ts})"));
        }
        message
    }
}

fn compute_timeout(
    now: Instant,
    dirty_since: Option<Instant>,
    debounce: Duration,
    last_poll: Instant,
    poll_interval: Duration,
    backoff_until: Option<Instant>,
) -> Duration {
    let mut deadline = now + Duration::from_secs(300);

    if let Some(until) = backoff_until {
        deadline = deadline.min(until);
    }

    if let Some(dirty_at) = dirty_since {
        let dirty_deadline = dirty_at + debounce;
        deadline = deadline.min(dirty_deadline);
    }

    let poll_deadline = last_poll + poll_interval;
    deadline = deadline.min(poll_deadline);

    deadline
        .saturating_duration_since(now)
        .min(Duration::from_secs(300))
        .max(Duration::from_millis(200))
}

fn backoff_delay(step: u32) -> Duration {
    let seconds = 1u64 << step;
    let base = Duration::from_secs(seconds);
    base.min(Duration::from_secs(300))
}
