# ObsyncGit

Lightweight watcher daemon that keeps a working tree in sync with a remote Git repository. It runs in the background, turns local edits into small commits, and periodically pulls remote changes made on your other machines.

## Features
- Watches a directory (recursively) and debounces edits before staging them.
- Generates readable commit messages such as `auto: Issues.md` or `auto: updated 4 files`.
- Rebases onto the remote branch before every push to minimise merge conflicts.
- Periodically pulls remote changes even when nothing happens locally.
- Configurable ignore globs so editor caches and similar junk stay out of Git.
- Emits compact structured logs via `tracing`.
- Optional desktop control centre with system tray integration (obsyncgit-gui).

## Installation

### Linux & macOS (curl | sh)

```bash
sh -c "$(curl -fsSL https://raw.githubusercontent.com/GezzyDax/ObsyncGit/main/scripts/install.sh)"
```

The installer auto-detects your platform (x86_64 Linux, Intel macOS, or Apple Silicon macOS) and installs the latest release into `/usr/local/bin` by default. Override the target version or destination with environment variables:

```bash
OBSYNCGIT_VERSION=v1.2.3 OBSYNCGIT_INSTALL_DIR=$HOME/.local/bin \
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/GezzyDax/ObsyncGit/main/scripts/install.sh)"
```

After installation the script registers a per-user login service (systemd user unit on Linux, LaunchAgent on macOS) so the daemon starts automatically. You can adjust or disable it later through your platform tools or via the `obsyncgit-gui` helper.

### Windows (PowerShell)

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/GezzyDax/ObsyncGit/main/scripts/install.ps1 | iex"
```

By default this installs the binary into `%LOCALAPPDATA%\ObsyncGit\bin` and makes sure the folder is on your user `PATH`. Set `OBSYNCGIT_VERSION` or `OBSYNCGIT_INSTALL_DIR` beforehand to customise the release tag or destination.

The installer also provisions a Task Scheduler entry named **ObsyncGit** so the daemon launches at logon.

### Build from source

```bash
cargo build --release
cp target/release/obsyncgit ~/.local/bin/
```

### Quick start

```bash
# 1. Create a starter configuration (overwrites with --force)
obsyncgit install

# 2. Edit the printed path and fill in repo_url / workdir

# 3. Launch the daemon
obsyncgit run
```

To stop the daemon press `Ctrl+C`; it shuts down cleanly.

### Desktop control centre

`obsyncgit-gui` ships alongside the daemon. It mimics the macOS visual style and works on Linux (Wayland/X11), macOS, and Windows. Use it to edit the YAML configuration, change author details, point to a dedicated SSH key, toggle automatic updates, or trigger a manual update. Closing the window hides it in the system tray; use the tray menu to restore or quit.

```
obsyncgit-gui              # launch the desktop helper
```

#### Linux GUI dependencies

The helper needs GTK and AppIndicator libraries at runtime. The installer tries to add them automatically, but here are the equivalent manual commands:

| Distro | Command |
| --- | --- |
| Ubuntu/Debian | `sudo apt-get install pkg-config libgtk-3-dev libglib2.0-dev libgobject-2.0-dev libgirepository1.0-dev libayatana-appindicator3-dev libxdo-dev` |
| Arch/Manjaro | `sudo pacman -S gtk3 glib2 gobject-introspection libappindicator-gtk3 xdotool` |
| Fedora/RHEL | `sudo dnf install gtk3 glib2 glib2-devel gobject-introspection gobject-introspection-devel libappindicator-gtk3 xdotool` |
| openSUSE | `sudo zypper install gtk3 glib2-devel gobject-introspection-devel libappindicator3-1 xdotool` |
| Alpine | `sudo apk add gtk+3.0 glib-dev gobject-introspection libappindicator3 xdotool` |
| Void | `sudo xbps-install gtk+3 glib-devel gobject-introspection libayatana-appindicator xdotool` |
| NixOS | `nix profile install nixpkgs#gtk3 nixpkgs#libayatana-appindicator nixpkgs#xdotool` |

### Install as a systemd user service (Linux)
1. Copy the release binary somewhere on your `$PATH`, e.g. `~/.local/bin/obsyncgit`.
2. Copy the supplied unit file and adjust the paths:
   - `cp examples/obsyncgit.service ~/.config/systemd/user/`
   - edit the `ExecStart` and `WorkingDirectory` lines.
3. Reload and enable:
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable --now obsyncgit.service
   ```
4. Inspect logs with `journalctl --user -u obsyncgit -f`.

macOS users can adapt the binary for `launchd` (see `examples/obsyncgit.plist`) and Windows users can register it through Task Scheduler or `nssm`.

## Configuration

`obsyncgit install` writes a starter YAML config to the default location (see output). To manage it afterwards:

```bash
# show the currently resolved configuration
obsyncgit settings show

# toggle automatic binary updates
obsyncgit settings set self-update.enabled true

# change the configured repository URL
obsyncgit settings set repo-url git@github.com:you/vault.git
```

You can still edit the YAML manually if you prefer. All paths must be absolute.

```yaml
repo_url: "git@github.com:you/vault.git"
branch: "main"
remote: "origin"
workdir: "/home/you/Obsidian"
debounce_seconds: 5
poll_interval_seconds: 180
commit:
  prefix: "auto:"
  max_files_in_summary: 5
  include_timestamp: true
ignore:
  globs:
    - ".obsidian/cache/**"
    - "**/*.tmp"
self_update:
  enabled: true
  command: null
  interval_hours: 24
git:
  author_name: "Vault Sync"
  author_email: "sync@example.com"
  ssh_key_path: "~/.ssh/id_ed25519"
```

Field notes:
- `repo_url`: SSH or HTTPS remotes work. The daemon runs `git remote set-url` if needed.
- `workdir`: Must either be an empty directory or an existing clone of `repo_url`.
- `debounce_seconds`: Minimum idle time before a commit is attempted.
- `poll_interval_seconds`: How often to `git pull --rebase` when no local edits happen.
- `commit.max_files_in_summary`: controls how many filenames appear in commit messages. Above that limit the message switches to `updated N files`.
- `ignore.globs`: Standard glob patterns matched against paths relative to `workdir`.
- `self_update`: Controls automatic binary updates. When enabled (default via CLI) ObsyncGit checks the GitHub releases page every `interval_hours` and replaces itself with the latest asset. Provide a `command` to run your own update script instead.
- `git`: Optional overrides for author/committer identity and the SSH key used when talking to the remote (`ssh_key_path`).

## Behaviour details
- New files are automatically staged thanks to `git add -A`.
- Commits are only produced when `git status --short` reports changes. If nothing is pending the daemon just performs periodic pulls.
- On rebase conflicts the daemon aborts the rebase and backs off exponentially; manual intervention is then required.
- Git commands run with `GIT_TERMINAL_PROMPT=0`, so configure SSH keys/credentials beforehand.

## Troubleshooting
- Run with `OBSYNCGIT_LOG=debug` to see every git invocation.
- Ensure the repository has sane permissions; the daemon does not sudo or elevate.
- Large binary files should be excluded with `.gitignore` or added to `ignore.globs`.

## Command line summary

```
obsyncgit run [--config path]              # start the daemon (default command)
obsyncgit install [--config path] [--force]
obsyncgit update [--config path] [--force]
obsyncgit-gui [--config path]              # desktop helper & tray
obsyncgit settings show|set KEY VALUE
obsyncgit --help
```

`--config` always points at an alternate YAML file; omit it to use the default in `~/.config/ObsyncGit/config.yaml` (or the platform equivalent). Keys accepted by `settings set` include `repo-url`, `branch`, `remote`, `workdir`, `self-update.enabled`, `self-update.interval-hours`, and `self-update.command`.

Run `obsyncgit update --force` to trigger a one-off update when automatic updates are disabled.

## Releases & auto-updates

Pushing a tag matching `v*` triggers the `release` GitHub Actions workflow. It now builds and packages binaries for:
- Linux x86_64 (`obsyncgit-linux-x86_64.tar.gz`)
- Linux ARM64 (`obsyncgit-linux-aarch64.tar.gz`)
- macOS Intel (`obsyncgit-macos-x86_64.tar.gz`)
- macOS Apple Silicon (`obsyncgit-macos-arm64.tar.gz`)
- Windows x86_64 (`obsyncgit-windows-x86_64.zip`)
- Windows ARM64 (`obsyncgit-windows-arm64.zip`)

Both the cross-platform installers (`install.sh` / `install.ps1`) and the in-app self-updater pull these assets directly, so keep the `obsyncgit-<target>.<ext>` naming if you add more targets.

Each archive bundles the daemon (`obsyncgit`) and the GUI helper (`obsyncgit-gui`) for the supported platform so the installer can place both.

## Project workflow

We use a Gitflow-style branching strategy:
- `main` holds the latest stable release, while `develop` is the integration branch for upcoming work.
- Feature branches (`feature/*`, `bugfix/*`, `hotfix/*`) always merge through pull requests; direct pushes to long-lived branches are discouraged.
- release-please automates version bumps, changelog entries, and tagging after changes land on `main`.

See `CONTRIBUTING.md` for full details on branching, PR expectations, and release management.
