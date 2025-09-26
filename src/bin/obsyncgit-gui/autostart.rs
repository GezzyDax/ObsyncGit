use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};
use camino::Utf8Path;
use directories::BaseDirs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutostartState {
    Enabled,
    Disabled,
    Unsupported,
}

fn daemon_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "obsyncgit.exe"
    } else {
        "obsyncgit"
    }
}

pub fn status() -> Result<AutostartState> {
    platform::status()
}

pub fn set_enabled(config_path: &Utf8Path, enabled: bool) -> Result<()> {
    platform::set_enabled(config_path, enabled)
}

fn find_daemon_binary() -> Result<PathBuf> {
    // Prefer a binary that lives alongside the GUI executable.
    let current_exe =
        std::env::current_exe().context("failed to determine current executable path")?;
    if let Some(dir) = current_exe.parent() {
        let candidate = dir.join(daemon_name());
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Fall back to searching PATH manually without additional dependencies.
    if let Some(paths) = std::env::var_os("PATH") {
        for entry in std::env::split_paths(&paths) {
            let candidate = entry.join(daemon_name());
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(anyhow!("could not locate obsyncgit daemon binary"))
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::fs;

    const SERVICE_NAME: &str = "obsyncgit.service";

    pub(super) fn status() -> Result<AutostartState> {
        let mut cmd = Command::new("systemctl");
        cmd.args(["--user", "is-enabled", SERVICE_NAME])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = match cmd.output() {
            Ok(output) => output,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AutostartState::Unsupported);
            }
            Err(err) => return Err(err).context("failed to invoke systemctl"),
        };

        if output.status.success() {
            return Ok(AutostartState::Enabled);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Failed to connect to bus") {
            return Ok(AutostartState::Unsupported);
        }

        Ok(AutostartState::Disabled)
    }

    pub(super) fn set_enabled(config_path: &Utf8Path, enabled: bool) -> Result<()> {
        let daemon = find_daemon_binary()?;
        let service_dir = determine_service_dir()?;
        fs::create_dir_all(&service_dir)
            .with_context(|| format!("failed to create {}", service_dir.display()))?;
        let unit_path = service_dir.join(SERVICE_NAME);

        if enabled {
            write_unit_file(&unit_path, &daemon, config_path)?;
            run_systemctl(["--user", "daemon-reload"])?;
            run_systemctl(["--user", "enable", "--now", SERVICE_NAME])?;
        } else {
            run_systemctl_allow_missing(["--user", "disable", "--now", SERVICE_NAME])?;
        }

        Ok(())
    }

    fn determine_service_dir() -> Result<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg).join("systemd/user"));
        }
        let dirs = BaseDirs::new().context("failed to determine home directory")?;
        Ok(dirs.home_dir().join(".config/systemd/user"))
    }

    fn write_unit_file(path: &Path, daemon: &Path, config_path: &Utf8Path) -> Result<()> {
        let exec_path = systemd_escape(&daemon.to_string_lossy());
        let config_value = systemd_escape(config_path.as_str());
        let contents = format!(
            "[Unit]\nDescription=ObsyncGit daemon\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nExecStart={exec} run\nEnvironment=RUST_LOG=info\nEnvironment=OBSYNCGIT_CONFIG={config}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
            exec = exec_path,
            config = config_value,
        );
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
    }

    fn systemd_escape(input: &str) -> String {
        let mut escaped = String::with_capacity(input.len() + 2);
        escaped.push('"');
        for ch in input.chars() {
            match ch {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                _ => escaped.push(ch),
            }
        }
        escaped.push('"');
        escaped
    }

    fn run_systemctl<I, S>(args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let status = Command::new("systemctl")
            .args(args)
            .status()
            .context("failed to invoke systemctl")?;
        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("systemctl command failed with status {status}"))
        }
    }

    fn run_systemctl_allow_missing<I, S>(args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new("systemctl")
            .args(args)
            .output()
            .context("failed to invoke systemctl")?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stderr}\n{stdout}");
        if combined.contains("does not exist") || combined.contains("not loaded") {
            return Ok(());
        }
        Err(anyhow!("systemctl command failed: {combined}"))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::fs;

    const LABEL: &str = "dev.obsyncgit.daemon";

    pub(super) fn status() -> Result<AutostartState> {
        let output = Command::new("launchctl").args(["list", LABEL]).output();
        match output {
            Ok(output) if output.status.success() => Ok(AutostartState::Enabled),
            Ok(_) => Ok(AutostartState::Disabled),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok(AutostartState::Unsupported)
            }
            Err(err) => Err(err).context("failed to invoke launchctl"),
        }
    }

    pub(super) fn set_enabled(config_path: &Utf8Path, enabled: bool) -> Result<()> {
        let daemon = find_daemon_binary()?;
        let plist_path = plist_path()?;
        if enabled {
            write_plist(&plist_path, &daemon, config_path)?;
            run_launchctl(["unload", &plist_path])?;
            run_launchctl(["load", "-w", &plist_path])?;
        } else {
            run_launchctl(["unload", "-w", &plist_path])?;
        }
        Ok(())
    }

    fn plist_path() -> Result<String> {
        let dirs = BaseDirs::new().context("failed to determine home directory")?;
        let path = dirs.home_dir().join("Library/LaunchAgents");
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        Ok(path
            .join("dev.obsyncgit.daemon.plist")
            .to_string_lossy()
            .into_owned())
    }

    fn write_plist(plist_path: &str, daemon: &Path, config_path: &Utf8Path) -> Result<()> {
        let logs_dir = Path::new(&plist_path)
            .parent()
            .context("launch agent path missing parent")?
            .parent()
            .context("failed to resolve Library directory")?
            .join("Logs");
        fs::create_dir_all(&logs_dir)
            .with_context(|| format!("failed to create {}", logs_dir.display()))?;

        let stdout_path = logs_dir.join("obsyncgit.log");
        let stderr_path = logs_dir.join("obsyncgit.err.log");

        let contents = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple Computer//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n    <key>Label</key>\n    <string>{label}</string>\n    <key>ProgramArguments</key>\n    <array>\n      <string>{daemon}</string>\n      <string>run</string>\n      <string>--config</string>\n      <string>{config}</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>StandardOutPath</key>\n    <string>{stdout}</string>\n    <key>StandardErrorPath</key>\n    <string>{stderr}</string>\n    <key>EnvironmentVariables</key>\n    <dict>\n      <key>OBSYNCGIT_CONFIG</key>\n      <string>{config}</string>\n    </dict>\n  </dict>\n</plist>\n",
            label = LABEL,
            daemon = daemon.to_string_lossy(),
            config = config_path.as_str(),
            stdout = stdout_path.to_string_lossy(),
            stderr = stderr_path.to_string_lossy(),
        );
        fs::write(plist_path, contents).with_context(|| format!("failed to write {plist_path}"))
    }

    fn run_launchctl<I, S>(args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let status = Command::new("launchctl")
            .args(args)
            .status()
            .context("failed to invoke launchctl")?;
        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("launchctl command failed with status {status}"))
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    const TASK_NAME: &str = "ObsyncGit";

    pub(super) fn status() -> Result<AutostartState> {
        let output = Command::new("schtasks")
            .args(["/Query", "/TN", TASK_NAME, "/FO", "LIST"])
            .output();
        match output {
            Ok(ref output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.to_ascii_lowercase().contains("disabled") {
                    Ok(AutostartState::Disabled)
                } else {
                    Ok(AutostartState::Enabled)
                }
            }
            Ok(_) => Ok(AutostartState::Disabled),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok(AutostartState::Unsupported)
            }
            Err(err) => Err(err).context("failed to invoke schtasks"),
        }
    }

    pub(super) fn set_enabled(config_path: &Utf8Path, enabled: bool) -> Result<()> {
        if enabled {
            register_task(config_path)?;
            change_task_state("/ENABLE")
        } else {
            change_task_state("/DISABLE")
        }
    }

    fn register_task(config_path: &Utf8Path) -> Result<()> {
        let daemon = find_daemon_binary()?;
        let command = format!(
            "\"{}\" run --config \"{}\"",
            daemon.to_string_lossy(),
            config_path.as_str()
        );

        // Remove existing task if present to ensure consistent settings.
        let _ = Command::new("schtasks")
            .args(["/Delete", "/TN", TASK_NAME, "/F"])
            .status();

        let status = Command::new("schtasks")
            .args([
                "/Create", "/TN", TASK_NAME, "/TR", &command, "/SC", "ONLOGON", "/RL", "LIMITED",
                "/F",
            ])
            .status()
            .context("failed to create scheduled task")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("failed to register scheduled task: {status}"))
        }
    }

    fn change_task_state(flag: &str) -> Result<()> {
        let status = Command::new("schtasks")
            .args(["/Change", "/TN", TASK_NAME, flag])
            .status()
            .context("failed to modify scheduled task state")?;
        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("failed to update scheduled task: {status}"))
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub(super) fn status() -> Result<AutostartState> {
        Ok(AutostartState::Unsupported)
    }

    pub(super) fn set_enabled(_config_path: &Utf8Path, _enabled: bool) -> Result<()> {
        Err(anyhow!("autostart is not supported on this platform"))
    }
}
