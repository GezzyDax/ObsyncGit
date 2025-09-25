#![cfg(feature = "gui")]

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{Context, Result, anyhow};
use camino::Utf8PathBuf;
use obsyncgit::config::Config;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use slint::CloseRequestResponse;
use slint::ComponentHandle;

slint::include_modules!();

struct AppState {
    config_path: Utf8PathBuf,
    config: Config,
}

fn main() -> Result<()> {
    let (config, config_path) =
        Config::detect_and_load(None).context("failed to load configuration")?;

    let state = Arc::new(Mutex::new(AppState {
        config_path,
        config,
    }));

    let ui = ConfiguratorWindow::new().context("failed to initialize UI")?;
    populate_ui(&ui, &state)?;

    let ui_weak_save = ui.as_weak();
    {
        let state = state.clone();
        ui.on_save_requested(move || {
            if let Some(ui) = ui_weak_save.upgrade()
                && let Err(err) = handle_save(&ui, state.clone())
            {
                set_status(&ui, format!("Save failed: {err}"));
            }
        });
    }

    let ui_weak_manual = ui.as_weak();
    ui.on_manual_update_requested(move || {
        if let Some(ui) = ui_weak_manual.upgrade() {
            match run_manual_update() {
                Ok(_) => set_status(&ui, "Manual update triggered"),
                Err(err) => set_status(&ui, format!("Manual update failed: {err}")),
            }
        }
    });

    ui.on_exit_requested(|| {
        std::process::exit(0);
    });

    setup_tray(&ui)?;

    ui.run()?;
    Ok(())
}

fn populate_ui(ui: &ConfiguratorWindow, state: &Arc<Mutex<AppState>>) -> Result<()> {
    let guard = state.lock().unwrap();
    ui.set_repo_url(guard.config.repo_url.clone().into());
    ui.set_branch(guard.config.branch.clone().into());
    ui.set_remote(guard.config.remote.clone().into());
    ui.set_workdir(guard.config.workdir.to_string().into());
    ui.set_author_name(
        guard
            .config
            .git
            .author_name
            .clone()
            .unwrap_or_default()
            .into(),
    );
    ui.set_author_email(
        guard
            .config
            .git
            .author_email
            .clone()
            .unwrap_or_default()
            .into(),
    );
    ui.set_ssh_key_path(
        guard
            .config
            .git
            .ssh_key_path
            .clone()
            .unwrap_or_default()
            .into(),
    );
    ui.set_auto_update_enabled(guard.config.self_update.enabled);
    ui.set_auto_update_interval_text(
        guard
            .config
            .self_update
            .interval_hours
            .unwrap_or(24)
            .to_string()
            .into(),
    );
    ui.set_status_text("".into());
    Ok(())
}

fn handle_save(ui: &ConfiguratorWindow, state: Arc<Mutex<AppState>>) -> Result<()> {
    let mut guard = state.lock().unwrap();
    guard.config.repo_url = ui.get_repo_url().into();
    guard.config.branch = ui.get_branch().into();
    guard.config.remote = ui.get_remote().into();
    guard.config.workdir = ui.get_workdir().to_string().into();

    let author_name = ui.get_author_name();
    guard.config.git.author_name = if author_name.is_empty() {
        None
    } else {
        Some(author_name.into())
    };

    let author_email = ui.get_author_email();
    guard.config.git.author_email = if author_email.is_empty() {
        None
    } else {
        Some(author_email.into())
    };

    let ssh_key = ui.get_ssh_key_path();
    guard.config.git.ssh_key_path = if ssh_key.is_empty() {
        None
    } else {
        Some(ssh_key.into())
    };

    guard.config.self_update.enabled = ui.get_auto_update_enabled();
    let interval_text = ui.get_auto_update_interval_text();
    let parsed = interval_text
        .parse::<u64>()
        .unwrap_or(guard.config.self_update.interval_hours.unwrap_or(24));
    let normalized_interval = parsed.max(1);
    guard.config.self_update.interval_hours = Some(normalized_interval);

    guard
        .config
        .save_to_path(&guard.config_path)
        .context("failed to write configuration")?;

    drop(guard);
    ui.set_auto_update_interval_text(normalized_interval.to_string().into());
    set_status(
        ui,
        format!("Saved at {}", humantime::format_rfc3339(SystemTime::now())),
    );
    Ok(())
}

fn run_manual_update() -> Result<()> {
    let status = std::process::Command::new("obsyncgit")
        .arg("update")
        .arg("--force")
        .status()
        .context("failed to spawn obsyncgit for update")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("obsyncgit update exited with status {status}"))
    }
}

fn set_status(ui: &ConfiguratorWindow, message: impl Into<String>) {
    ui.set_status_text(message.into().into());
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn setup_tray(window: &ConfiguratorWindow) -> Result<()> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{TrayIconBuilder, TrayIconEvent};

    let window_handle = window.window();
    let window_weak = window.as_weak();
    window_handle.on_close_requested(move || {
        if let Some(ui) = window_weak.upgrade() {
            let _ = ui.window().hide();
        }
        CloseRequestResponse::HideWindow
    });

    let tray_icon = load_tray_icon()?;

    let menu = Menu::new();
    let show_item = Box::leak(Box::new(MenuItem::new("Show", true, None)));
    let quit_item = Box::leak(Box::new(MenuItem::new("Quit", true, None)));
    menu.append_items(&[show_item, quit_item])?;

    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    let tray = TrayIconBuilder::new()
        .with_tooltip("ObsyncGit")
        .with_icon(tray_icon)
        .with_menu(Box::new(menu))
        .build()?;
    let tray_id = tray.id().clone();

    let window_for_menu = window.as_weak();
    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver().clone();
        for event in receiver.iter() {
            if event.id == show_id {
                let weak = window_for_menu.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        let _ = ui.window().show();
                    }
                });
            } else if event.id == quit_id {
                std::process::exit(0);
            }
        }
    });

    let window_for_tray = window.as_weak();
    std::thread::spawn(move || {
        let receiver = TrayIconEvent::receiver().clone();
        for event in receiver.iter() {
            if event.id != tray_id {
                continue;
            }
            let weak = window_for_tray.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let window = ui.window();
                    let _ = window.show();
                }
            });
        }
    });

    // Keep tray icon alive
    std::mem::forget(tray);

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn setup_tray(_window: &ConfiguratorWindow) -> Result<()> {
    tracing::warn!("Tray icon support is currently unavailable on this platform");
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn load_tray_icon() -> Result<tray_icon::Icon> {
    let bytes = include_bytes!("../../assets/tray-icon.png");
    let image = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(tray_icon::Icon::from_rgba(image.into_raw(), width, height)?)
}
