use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use tracing::{debug, warn};

use crate::config::{Config, GitOptions};

#[derive(Debug, Clone)]
pub struct GitFacade {
    executable: String,
    repo_path: PathBuf,
    remote: String,
    branch: String,
    git_options: GitOptions,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
}

impl GitFacade {
    pub fn new(config: &Config) -> Result<Self> {
        let exe = config
            .git
            .executable
            .clone()
            .unwrap_or_else(|| "git".to_string());
        Ok(Self {
            executable: exe,
            repo_path: config.workdir.clone().into_std_path_buf(),
            remote: config.remote.clone(),
            branch: config.branch.clone(),
            git_options: config.git.clone(),
        })
    }

    pub fn ensure_repo(&self, repo_url: &str) -> Result<()> {
        if self.repo_path.join(".git").exists() {
            debug!(path = %self.repo_path.display(), "repository already present, refreshing configuration");
            self.set_remote(repo_url)?;
            self.fetch()?;
            self.checkout_branch()?;
            return Ok(());
        }

        if self.repo_path.exists() {
            let mut entries = std::fs::read_dir(&self.repo_path).with_context(|| {
                format!(
                    "failed to inspect existing directory {}",
                    self.repo_path.display()
                )
            })?;
            if entries.next().is_some() {
                bail!(
                    "target directory {} is not empty and does not contain a git repository",
                    self.repo_path.display()
                );
            }
        } else if let Some(parent) = self.repo_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create parent directory for {}",
                    self.repo_path.display()
                )
            })?;
        }
        std::fs::create_dir_all(&self.repo_path).with_context(|| {
            format!(
                "failed to create repository directory {}",
                self.repo_path.display()
            )
        })?;

        self.clone_repo(repo_url)?;
        self.checkout_branch()?;
        Ok(())
    }

    fn clone_repo(&self, repo_url: &str) -> Result<()> {
        debug!(url = repo_url, path = %self.repo_path.display(), "Cloning repository");
        let args = ["clone", "--branch", &self.branch, repo_url, "."];
        self.run_git(&args, false).context("git clone failed")?;
        Ok(())
    }

    fn set_remote(&self, repo_url: &str) -> Result<()> {
        let result = self.run_git(&["remote", "get-url", &self.remote], false);
        match result {
            Ok(current) => {
                let current_url = current.stdout.trim();
                if current_url != repo_url {
                    debug!(remote = %self.remote, url = repo_url, "Updating remote URL");
                    self.run_git(&["remote", "set-url", &self.remote, repo_url], false)?;
                }
            }
            Err(_) => {
                debug!(remote = %self.remote, url = repo_url, "Adding missing remote");
                self.run_git(&["remote", "add", &self.remote, repo_url], false)?;
            }
        }
        Ok(())
    }

    pub fn fetch(&self) -> Result<()> {
        self.run_git(&["fetch", &self.remote], false)?;
        Ok(())
    }

    pub fn checkout_branch(&self) -> Result<()> {
        if let Ok(output) = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"], false)
            && output.stdout.trim() == self.branch
        {
            return Ok(());
        }

        if let Err(err) = self.run_git(&["checkout", &self.branch], false) {
            debug!(
                ?err,
                "branch checkout failed, attempting to create tracking branch"
            );
            let remote_ref = format!("{}/{}", self.remote, self.branch);
            self.run_git(&["checkout", "-b", &self.branch, &remote_ref], false)
                .context("failed to create tracking branch")?;
        }
        Ok(())
    }

    pub fn list_changed_files(&self) -> Result<Vec<String>> {
        let status = self.run_git(&["status", "--short"], false)?;
        let mut files = Vec::new();
        for line in status.stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let payload = if line.len() > 3 { &line[3..] } else { line };
            let path = if let Some(pos) = payload.rfind(" -> ") {
                &payload[pos + 4..]
            } else {
                payload
            };
            files.push(path.trim().to_string());
        }
        Ok(files)
    }

    pub fn stage_all(&self) -> Result<()> {
        self.run_git(&["add", "-A"], false)?;
        Ok(())
    }

    fn worktree_status(&self) -> Result<String> {
        let status = self.run_git(&["status", "--porcelain"], false)?;
        Ok(status.stdout)
    }

    fn is_worktree_clean(&self) -> Result<bool> {
        Ok(self.worktree_status()?.trim().is_empty())
    }

    fn ensure_autostash(&self) -> Result<Option<String>> {
        if self.is_worktree_clean()? {
            return Ok(None);
        }

        const STASH_MESSAGE: &str = "obsyncgit-autostash";

        self.run_git(
            &[
                "stash",
                "push",
                "--include-untracked",
                "--message",
                STASH_MESSAGE,
            ],
            false,
        )
        .context("failed to stash local changes before pull --rebase")?;

        let list = self
            .run_git(&["stash", "list", "--format=%gd:%gs"], false)
            .context("failed to inspect git stash after push")?;

        for line in list.stdout.lines() {
            if let Some((stash_ref, message)) = line.split_once(':')
                && message.trim() == STASH_MESSAGE
            {
                return Ok(Some(stash_ref.trim().to_string()));
            }
        }

        // Fallback: assume newest stash (stash@{0}) belongs to us.
        Ok(Some("stash@{0}".to_string()))
    }

    fn pop_stash(&self, stash_ref: &str) {
        if let Err(err) = self.run_git(&["stash", "pop", stash_ref], false) {
            warn!(?err, "failed to restore stash after pull --rebase");
        }
    }

    pub fn commit(&self, message: &str) -> Result<bool> {
        let status = self.run_git(&["status", "--short"], false)?;
        if status.stdout.trim().is_empty() {
            return Ok(false);
        }
        self.run_git(&["commit", "-m", message], true)?;
        Ok(true)
    }

    pub fn pull_rebase(&self) -> Result<()> {
        let autostash = self.ensure_autostash()?;
        let result = self.run_git(&["pull", "--rebase", &self.remote, &self.branch], false);

        match result {
            Ok(_) => {
                if let Some(stash_ref) = autostash {
                    self.pop_stash(&stash_ref);
                }
                Ok(())
            }
            Err(err) => {
                warn!(?err, "git pull --rebase failed, attempting to abort rebase");
                let _ = self.run_git(&["rebase", "--abort"], false);
                if let Some(stash_ref) = autostash {
                    self.pop_stash(&stash_ref);
                }
                Err(err)
            }
        }
    }

    pub fn push(&self) -> Result<()> {
        self.run_git(&["push", &self.remote, &self.branch], false)?;
        Ok(())
    }

    fn run_git(&self, args: &[&str], include_author_env: bool) -> Result<CommandOutput> {
        debug!(cmd = ?args, "running git command");
        let mut cmd = Command::new(&self.executable);
        cmd.current_dir(&self.repo_path)
            .arg("-c")
            .arg("core.quotepath=false")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env("LANG", "C");

        if include_author_env {
            if let Some(name) = &self.git_options.author_name {
                cmd.env("GIT_AUTHOR_NAME", name)
                    .env("GIT_COMMITTER_NAME", name);
            }
            if let Some(email) = &self.git_options.author_email {
                cmd.env("GIT_AUTHOR_EMAIL", email)
                    .env("GIT_COMMITTER_EMAIL", email);
            }
        }

        let output = cmd
            .output()
            .with_context(|| format!("failed to execute git command: git {}", join_args(args)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.trim().is_empty() {
            debug!(stderr = %stderr.trim(), cmd = %join_args(args), "git stderr");
        }

        if !output.status.success() {
            let code = output.status.code().unwrap_or(-1);
            return Err(anyhow!(
                "git {} failed with code {}: {}{}",
                join_args(args),
                code,
                if !stderr.is_empty() {
                    stderr.clone()
                } else {
                    String::new()
                },
                if !stdout.is_empty() && stderr.is_empty() {
                    format!(" stdout: {stdout}")
                } else {
                    String::new()
                }
            ));
        }

        Ok(CommandOutput { stdout, stderr })
    }
}

fn join_args(args: &[&str]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                (*arg).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
