use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use tracing::{debug, info, warn};

use crate::config::SelfUpdateConfig;

const REPO_OWNER: &str = "GezzyDax";
const REPO_NAME: &str = "ObsyncGit";
const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct SelfUpdateManager {
    config: SelfUpdateConfig,
    config_path: Utf8PathBuf,
}

impl SelfUpdateManager {
    pub fn new(config: &SelfUpdateConfig, config_path: &Utf8Path) -> Self {
        Self {
            config: config.clone(),
            config_path: config_path.to_owned(),
        }
    }

    pub fn spawn_if_enabled(
        config: &SelfUpdateConfig,
        config_path: &Utf8Path,
        shutdown: Arc<AtomicBool>,
    ) -> Option<thread::JoinHandle<()>> {
        if !config.enabled {
            return None;
        }
        Some(Self::new(config, config_path).spawn(shutdown))
    }

    pub fn spawn(self, shutdown: Arc<AtomicBool>) -> thread::JoinHandle<()> {
        let interval_hours = self.config.interval_hours.unwrap_or(24).max(1);
        let sleep_interval = Duration::from_secs(interval_hours * 3600);
        thread::Builder::new()
            .name("obsyncgit-self-update".to_string())
            .spawn(move || {
                debug!(path = %self.config_path, "self-update worker started");
                if let Err(err) = self.check_now(false) {
                    warn!(?err, "initial self-update check failed");
                }
                loop {
                    if sleep_interval == Duration::from_secs(0) {
                        break;
                    }
                    let target = Instant::now() + sleep_interval;
                    while Instant::now() < target {
                        if shutdown.load(Ordering::SeqCst) {
                            debug!("self-update worker stopping");
                            return;
                        }
                        let now = Instant::now();
                        if now >= target {
                            break;
                        }
                        let remaining = target - now;
                        thread::sleep(remaining.min(Duration::from_secs(60)));
                    }
                    if shutdown.load(Ordering::SeqCst) {
                        debug!("self-update worker stopping");
                        return;
                    }
                    if let Err(err) = self.check_now(false) {
                        warn!(?err, "scheduled self-update check failed");
                    }
                }
            })
            .expect("self-update worker thread")
    }

    pub fn check_now(&self, force: bool) -> Result<()> {
        if force {
            debug!("forced self-update check requested");
        }
        if let Some(cmd) = &self.config.command {
            run_custom_command(cmd, force)
        } else {
            self.run_default_updater()
        }
    }

    fn run_default_updater(&self) -> Result<()> {
        let status = self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .current_version(CURRENT_VERSION)
            .build()
            .context("failed to configure GitHub self-update")?
            .update()
            .context("failed to execute GitHub self-update")?;

        match status {
            self_update::Status::Updated(version) => {
                info!(%version, "obsyncgit updated to new version");
            }
            self_update::Status::UpToDate(version) => {
                debug!(%version, "obsyncgit already up to date");
            }
        }
        debug!(path = %self.config_path, "self-update check complete");
        Ok(())
    }
}

fn run_custom_command(command: &str, _force: bool) -> Result<()> {
    info!(%command, "running custom self-update command");
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()
        .with_context(|| format!("failed to spawn self-update command: {command}"))?;
    if status.success() {
        info!("custom self-update command finished successfully");
        Ok(())
    } else {
        Err(anyhow!("self-update command exited with status {}", status))
    }
}
