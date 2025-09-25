use std::{fs, time::Duration};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use serde::Deserialize;

fn default_branch() -> String {
    "main".to_string()
}

fn default_remote() -> String {
    "origin".to_string()
}

fn default_debounce_seconds() -> u64 {
    5
}

fn default_poll_interval_seconds() -> u64 {
    300
}

fn default_commit_prefix() -> String {
    "auto:".to_string()
}

fn default_max_files_in_summary() -> usize {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub repo_url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_remote")]
    pub remote: String,
    pub workdir: Utf8PathBuf,
    #[serde(default = "default_debounce_seconds")]
    pub debounce_seconds: u64,
    #[serde(default = "default_poll_interval_seconds")]
    pub poll_interval_seconds: u64,
    #[serde(default)]
    pub commit: CommitConfig,
    #[serde(default)]
    pub ignore: IgnoreConfig,
    #[serde(default)]
    pub self_update: SelfUpdateConfig,
    #[serde(default)]
    pub git: GitOptions,
}

impl Config {
    pub fn load_from_path<P: AsRef<Utf8Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file at {path}"))?;
        let mut config: Config = serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse YAML config at {path}"))?;
        config.normalize();
        Ok(config)
    }

    pub fn detect_and_load(explicit: Option<Utf8PathBuf>) -> Result<(Self, Utf8PathBuf)> {
        if let Some(path) = explicit {
            let cfg = Self::load_from_path(&path)?;
            return Ok((cfg, path));
        }

        if let Ok(env_path) = std::env::var("GIT_SYNCD_CONFIG") {
            let utf_path = Utf8PathBuf::from(env_path);
            let cfg = Self::load_from_path(&utf_path)?;
            return Ok((cfg, utf_path));
        }

        let project_dirs = ProjectDirs::from("dev", "git-syncd", "git-syncd")
            .context("cannot determine default config directory")?;
        let default_path =
            Utf8PathBuf::from_path_buf(project_dirs.config_dir().join("config.yaml"))
                .ok()
                .context("default config path is not valid UTF-8")?;
        let cfg = Self::load_from_path(&default_path)?;
        Ok((cfg, default_path))
    }

    pub fn debounce_duration(&self) -> Duration {
        Duration::from_secs(self.debounce_seconds.max(1))
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_seconds.max(30))
    }

    fn normalize(&mut self) {
        if self.commit.prefix.trim().is_empty() {
            self.commit.prefix = default_commit_prefix();
        }
        if self.commit.max_files_in_summary == 0 {
            self.commit.max_files_in_summary = default_max_files_in_summary();
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitConfig {
    #[serde(default = "default_commit_prefix")]
    pub prefix: String,
    #[serde(default = "default_max_files_in_summary")]
    pub max_files_in_summary: usize,
    #[serde(default)]
    pub include_timestamp: bool,
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            prefix: default_commit_prefix(),
            max_files_in_summary: default_max_files_in_summary(),
            include_timestamp: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IgnoreConfig {
    #[serde(default)]
    pub globs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SelfUpdateConfig {
    pub enabled: bool,
    pub command: Option<String>,
    pub interval_hours: Option<u64>,
}

impl Default for SelfUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: None,
            interval_hours: Some(24),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GitOptions {
    pub executable: Option<String>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

impl Default for GitOptions {
    fn default() -> Self {
        Self {
            executable: None,
            author_name: None,
            author_email: None,
        }
    }
}
