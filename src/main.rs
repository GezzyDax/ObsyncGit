use std::str::FromStr;
use std::sync::atomic::Ordering;

use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use obsyncgit::config::{CommitConfig, Config, GitOptions, IgnoreConfig, SelfUpdateConfig};
use obsyncgit::daemon::SyncDaemon;
use obsyncgit::updater::SelfUpdateManager;
use tracing::{info, warn};

const BIN_NAME: &str = env!("CARGO_BIN_NAME");

#[derive(Parser, Debug)]
#[command(name = BIN_NAME, version, about = "Obsidian Git synchronizer daemon")]
struct Cli {
    /// Path to configuration YAML file
    #[arg(global = true, short, long, value_name = "PATH")]
    config: Option<Utf8PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Run the background synchronizer (default)
    Run,
    /// Create a starter configuration file
    Install {
        /// Overwrite an existing file
        #[arg(long)]
        force: bool,
    },
    /// Manually trigger a binary self-update
    Update {
        /// Force the updater even if auto-updates are disabled
        #[arg(long)]
        force: bool,
    },
    /// Inspect or change configuration values
    Settings {
        #[command(subcommand)]
        command: SettingsCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum SettingsCommand {
    /// Print the resolved configuration as YAML
    Show,
    /// Update a configuration value (e.g. self-update.enabled true)
    Set { key: SettingsKey, value: String },
}

#[derive(Debug, Clone, Copy)]
enum SettingsKey {
    RepoUrl,
    Branch,
    Remote,
    Workdir,
    SelfUpdateEnabled,
    SelfUpdateIntervalHours,
    SelfUpdateCommand,
    GitSshKeyPath,
}

impl FromStr for SettingsKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.replace('_', "-").to_ascii_lowercase().as_str() {
            "repo-url" => Ok(Self::RepoUrl),
            "branch" => Ok(Self::Branch),
            "remote" => Ok(Self::Remote),
            "workdir" | "work-dir" => Ok(Self::Workdir),
            "self-update.enabled" | "self-update-enabled" => Ok(Self::SelfUpdateEnabled),
            "self-update.interval-hours" | "self-update-interval" | "self-update.interval" => {
                Ok(Self::SelfUpdateIntervalHours)
            }
            "self-update.command" | "self-update-command" => Ok(Self::SelfUpdateCommand),
            "git.ssh-key" | "git.ssh-key-path" | "ssh-key" => Ok(Self::GitSshKeyPath),
            other => Err(format!("unknown configuration key: {other}")),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging();

    let Cli { config, command } = cli;
    match command.unwrap_or(Command::Run) {
        Command::Run => handle_run(config),
        Command::Install { force } => handle_install(config, force),
        Command::Update { force } => handle_update(config, force),
        Command::Settings { command } => handle_settings(config, command),
    }
}

fn handle_run(config_arg: Option<Utf8PathBuf>) -> Result<()> {
    let (config, config_path) = Config::detect_and_load(config_arg.clone())?;
    info!(path = %config_path, "configuration loaded");

    let daemon = SyncDaemon::new(config.clone())?;
    let shutdown = daemon.shutdown_handle();
    let update_handle =
        SelfUpdateManager::spawn_if_enabled(&config.self_update, &config_path, shutdown.clone());

    daemon.run()?;
    shutdown.store(true, Ordering::SeqCst);
    if let Some(handle) = update_handle
        && let Err(err) = handle.join()
    {
        warn!(?err, "self-update worker exited unexpectedly");
    }
    Ok(())
}

fn handle_install(config_arg: Option<Utf8PathBuf>, force: bool) -> Result<()> {
    let path = Config::resolve_path(config_arg)?;
    if path.exists() && !force {
        bail!(
            "configuration already exists at {} (use --force to overwrite)",
            path
        );
    }
    let cfg = default_config();
    cfg.save_to_path(&path)?;
    println!("Created configuration at {path}. Edit this file before running `obsyncgit run`.");
    Ok(())
}

fn handle_update(config_arg: Option<Utf8PathBuf>, force: bool) -> Result<()> {
    let (config, config_path) = Config::detect_and_load(config_arg)?;
    if !config.self_update.enabled && !force {
        println!(
            "Auto-updates are disabled in the configuration. Re-run with --force or enable them via \"obsyncgit settings set self-update.enabled true\"."
        );
        return Ok(());
    }
    let manager = SelfUpdateManager::new(&config.self_update, &config_path);
    manager.check_now(force)?;
    println!("Self-update check completed.");
    if !config.self_update.enabled {
        println!(
            "Auto-updates are currently disabled. Enable them with `obsyncgit settings set self-update.enabled true` if desired."
        );
    }
    Ok(())
}

fn handle_settings(config_arg: Option<Utf8PathBuf>, command: SettingsCommand) -> Result<()> {
    match command {
        SettingsCommand::Show => {
            let (config, _) = Config::detect_and_load(config_arg)?;
            let rendered =
                serde_yaml::to_string(&config).context("failed to render configuration as YAML")?;
            println!("{rendered}");
            Ok(())
        }
        SettingsCommand::Set { key, value } => {
            let path = Config::resolve_path(config_arg.clone())?;
            let mut config = Config::load_from_path(&path)?;
            apply_setting(&mut config, key, &value)?;
            config.save_to_path(&path)?;
            println!("Updated {key:?} in {path}");
            Ok(())
        }
    }
}

fn apply_setting(config: &mut Config, key: SettingsKey, value: &str) -> Result<()> {
    match key {
        SettingsKey::RepoUrl => config.repo_url = value.to_string(),
        SettingsKey::Branch => config.branch = value.to_string(),
        SettingsKey::Remote => config.remote = value.to_string(),
        SettingsKey::Workdir => {
            if value.trim().is_empty() {
                bail!("workdir cannot be empty");
            }
            config.workdir = Utf8PathBuf::from(value);
        }
        SettingsKey::SelfUpdateEnabled => {
            config.self_update.enabled = parse_bool(value)?;
        }
        SettingsKey::SelfUpdateIntervalHours => {
            config.self_update.interval_hours = parse_optional_hours(value)?;
        }
        SettingsKey::SelfUpdateCommand => {
            let cleaned = value.trim();
            if cleaned.eq_ignore_ascii_case("none") || cleaned.is_empty() {
                config.self_update.command = None;
            } else {
                config.self_update.command = Some(cleaned.to_string());
            }
        }
        SettingsKey::GitSshKeyPath => {
            let cleaned = value.trim();
            if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("none") {
                config.git.ssh_key_path = None;
            } else {
                config.git.ssh_key_path = Some(cleaned.to_string());
            }
        }
    }
    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        other => bail!("cannot parse '{other}' as boolean"),
    }
}

fn parse_optional_hours(value: &str) -> Result<Option<u64>> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "none" || normalized == "never" || normalized == "off"
    {
        return Ok(None);
    }
    let hours: u64 = normalized
        .parse()
        .with_context(|| format!("failed to parse '{value}' as hours"))?;
    Ok(Some(hours))
}

fn default_config() -> Config {
    let workdir = BaseDirs::new()
        .and_then(|dirs| Utf8PathBuf::from_path_buf(dirs.home_dir().join("Obsidian")).ok())
        .unwrap_or_else(|| Utf8PathBuf::from("/path/to/your/obsidian-vault"));

    Config {
        repo_url: "git@github.com:username/repo.git".to_string(),
        branch: "main".to_string(),
        remote: "origin".to_string(),
        workdir,
        debounce_seconds: 5,
        poll_interval_seconds: 300,
        commit: CommitConfig::default(),
        ignore: IgnoreConfig {
            globs: vec![
                ".obsidian/cache/**".to_string(),
                "**/*.tmp".to_string(),
                "**/*.swp".to_string(),
            ],
        },
        self_update: SelfUpdateConfig {
            enabled: true,
            command: None,
            interval_hours: Some(24),
        },
        git: GitOptions::default(),
    }
}

fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = std::env::var("OBSYNCGIT_LOG")
        .or_else(|_| std::env::var("GIT_SYNCD_LOG"))
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string());

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .compact()
        .finish();

    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("failed to initialize logging: {err}");
    }
}
