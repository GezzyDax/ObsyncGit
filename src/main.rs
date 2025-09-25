mod config;
mod daemon;
mod git;
mod ignore;

use anyhow::{Context, Result, anyhow};
use camino::Utf8PathBuf;
use config::Config;
use daemon::SyncDaemon;

const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const APP_NAME: &str = env!("CARGO_PKG_NAME");

fn main() -> Result<()> {
    let config_path = parse_args()?;
    init_logging();

    let (config, resolved_path) = Config::detect_and_load(config_path)?;
    tracing::info!(path = %resolved_path, "configuration loaded");

    let daemon = SyncDaemon::new(config)?;
    daemon.run()?;
    Ok(())
}

fn parse_args() -> Result<Option<Utf8PathBuf>> {
    let mut args = std::env::args().skip(1);
    let mut config = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                let value = args.next().context("expected value after --config/-c")?;
                config = Some(Utf8PathBuf::from(value));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("{APP_NAME} {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => {
                return Err(anyhow!("unknown argument: {other}"));
            }
        }
    }

    Ok(config)
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

fn print_help() {
    println!("{APP_NAME} - lightweight git-based folder synchronizer");
    println!("\nUSAGE:\n    {BIN_NAME} [OPTIONS]\n");
    println!("OPTIONS:");
    println!("    -c, --config <PATH>    Path to configuration YAML file");
    println!("    -h, --help             Show this help message");
    println!("    -V, --version          Print version information");
}
