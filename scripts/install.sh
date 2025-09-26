#!/usr/bin/env sh
set -eu

REPO="GezzyDax/ObsyncGit"
PROJECT="obsyncgit"
DEFAULT_INSTALL_DIR="/usr/local/bin"
INSTALL_DIR="${OBSYNCGIT_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
VERSION="${OBSYNCGIT_VERSION:-latest}"
SKIP_DEP_INSTALL="${OBSYNCGIT_SKIP_DEPENDENCIES:-}" 

print_usage() {
  cat <<'USAGE'
Usage: install.sh [--version <tag>] [--install-dir <path>]

Environment variables:
  OBSYNCGIT_VERSION       Override release tag (defaults to latest)
  OBSYNCGIT_INSTALL_DIR   Override installation directory (defaults to /usr/local/bin)
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      shift
      [ "$#" -gt 0 ] || { echo "Missing value for --version" >&2; exit 1; }
      VERSION="$1"
      ;;
    --install-dir)
      shift
      [ "$#" -gt 0 ] || { echo "Missing value for --install-dir" >&2; exit 1; }
      INSTALL_DIR="$1"
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      print_usage
      exit 1
      ;;
  esac
  shift
done

for cmd in curl tar; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Required dependency '$cmd' is not installed" >&2
    exit 1
  fi
done

uname_s="$(uname -s)"
uname_m="$(uname -m)"

ASSET_EXT="tar.gz"
OS_ID=""
ARCH_ID=""

case "$uname_s" in
  Linux)
    OS_ID="linux"
    case "$uname_m" in
      x86_64|amd64)
        ARCH_ID="x86_64"
        ;;
      aarch64|arm64)
        ARCH_ID="aarch64"
        ;;
      armv7l|armv7*)
        echo "32-bit ARM Linux builds are not yet supported" >&2
        exit 1
        ;;
      *)
        echo "Unsupported Linux architecture: $uname_m" >&2
        exit 1
        ;;
    esac
    ;;
  Darwin)
    OS_ID="macos"
    case "$uname_m" in
      x86_64)
        ARCH_ID="x86_64"
        ;;
      arm64)
        ARCH_ID="arm64"
        ;;
      *)
        echo "Unsupported macOS architecture: $uname_m" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported operating system: $uname_s" >&2
    exit 1
    ;;
esac

ASSET_NAME="$PROJECT-$OS_ID-$ARCH_ID.$ASSET_EXT"

if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ASSET_NAME"
else
  case "$VERSION" in
    v*) ;;
    *) VERSION="v$VERSION" ;;
  esac
  DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET_NAME"
fi

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT INT HUP TERM

ARCHIVE_PATH="$TMPDIR/$ASSET_NAME"

echo "Downloading $DOWNLOAD_URL"
curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"

tar -C "$TMPDIR" -xzf "$ARCHIVE_PATH"

if [ ! -d "$INSTALL_DIR" ]; then
  if mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    :
  else
    if command -v sudo >/dev/null 2>&1; then
      echo "Creating $INSTALL_DIR with sudo"
      sudo mkdir -p "$INSTALL_DIR"
    else
      echo "Cannot create $INSTALL_DIR (insufficient permissions)" >&2
      exit 1
    fi
  fi
fi

install_file() {
  src="$1"
  dest="$INSTALL_DIR/$(basename "$src")"
  if [ -w "$INSTALL_DIR" ]; then
    install -m 755 "$src" "$dest"
  else
    if command -v sudo >/dev/null 2>&1; then
      sudo install -m 755 "$src" "$dest"
    else
      echo "Insufficient permissions to write to $INSTALL_DIR" >&2
      exit 1
    fi
  fi
  echo "Installed $(basename "$src") to $dest"
}

install_linux_runtime_deps() {
  if [ "$SKIP_DEP_INSTALL" = "1" ]; then
    echo "Skipping dependency installation as OBSYNCGIT_SKIP_DEPENDENCIES=1"
    return
  fi

  if [ "${GUI_INSTALLED:-0}" -ne 1 ]; then
    return
  fi

  SUDO_CMD=""
  if [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
      SUDO_CMD="sudo"
    else
      echo "Install GUI dependencies manually (requires root): gtk3, libayatana-appindicator, xdotool"
      return
    fi
  fi

  install_cmd() {
    if [ -n "$SUDO_CMD" ]; then
      "$SUDO_CMD" "$@"
    else
      "$@"
    fi
  }

  if command -v apt-get >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via apt-get"
    if ! install_cmd apt-get update; then
      echo "apt-get update failed; install GUI dependencies manually." >&2
      return
    fi

    APT_PACKAGES="pkg-config libgtk-3-dev libglib2.0-dev libgirepository1.0-dev libayatana-appindicator3-dev libxdo-dev"
    available_packages=""
    missing_packages=""

    for pkg in $APT_PACKAGES; do
      if apt-cache show "$pkg" >/dev/null 2>&1; then
        available_packages="${available_packages:+$available_packages }$pkg"
      else
        missing_packages="${missing_packages:+$missing_packages }$pkg"
      fi
    done

    if [ -n "$available_packages" ]; then
      if ! install_cmd apt-get install -y $available_packages; then
        echo "Please install manually: sudo apt-get install $available_packages" >&2
      fi
    else
      echo "None of the expected GUI packages are available via apt-get; install the required GTK/AppIndicator dependencies manually." >&2
    fi

    if [ -n "$missing_packages" ]; then
      echo "Skipped unavailable packages: $missing_packages" >&2
      echo "Install the closest alternatives provided by your distribution if the GUI requires them." >&2
    fi
  elif command -v pacman >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via pacman"
    if ! install_cmd pacman --noconfirm --needed -S gtk3 glib2 gobject-introspection libappindicator-gtk3 xdotool; then
      echo "Install manually: sudo pacman -S gtk3 glib2 gobject-introspection libappindicator-gtk3 xdotool"
    fi
  elif command -v dnf >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via dnf"
    if ! install_cmd dnf install -y gtk3 glib2 glib2-devel gobject-introspection gobject-introspection-devel libappindicator-gtk3 xdotool; then
      echo "Install manually: sudo dnf install gtk3 glib2 glib2-devel gobject-introspection gobject-introspection-devel libappindicator-gtk3 xdotool"
    fi
  elif command -v yum >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via yum"
    if ! install_cmd yum install -y gtk3 glib2 glib2-devel gobject-introspection gobject-introspection-devel libappindicator-gtk3 xdotool; then
      echo "Install manually: sudo yum install gtk3 glib2 glib2-devel gobject-introspection gobject-introspection-devel libappindicator-gtk3 xdotool"
    fi
  elif command -v zypper >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via zypper"
    if ! install_cmd zypper --non-interactive install gtk3 glib2-devel gobject-introspection-devel libappindicator3-1 xdotool; then
      echo "Install manually: sudo zypper install gtk3 glib2-devel gobject-introspection-devel libappindicator3-1 xdotool"
    fi
  elif command -v apk >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via apk"
    if ! install_cmd apk add --no-cache gtk+3.0 glib-dev gobject-introspection libappindicator3 xdotool; then
      echo "Install manually: sudo apk add gtk+3.0 glib-dev gobject-introspection libappindicator3 xdotool"
    fi
  elif command -v xbps-install >/dev/null 2>&1; then
    echo "Installing GUI runtime dependencies via xbps-install"
    if ! install_cmd xbps-install -Sy gtk+3 glib-devel gobject-introspection libayatana-appindicator xdotool; then
      echo "Install manually: sudo xbps-install gtk+3 glib-devel gobject-introspection libayatana-appindicator xdotool"
    fi
  elif command -v nix-env >/dev/null 2>&1; then
    echo "NixOS detected. Install GUI dependencies with: nix profile install nixpkgs#gtk3 nixpkgs#libayatana-appindicator nixpkgs#xdotool"
  else
    echo "Install GUI dependencies manually: GTK 3, GLib, GObject Introspection, libayatana-appindicator, xdotool"
  fi
}

FOUND_BINARIES=0
GUI_INSTALLED=0
for f in "$TMPDIR"/obsyncgit*; do
  if [ -f "$f" ] && [ -x "$f" ]; then
    install_file "$f"
    FOUND_BINARIES=$((FOUND_BINARIES + 1))
    case "$(basename "$f")" in
      obsyncgit-gui*)
        GUI_INSTALLED=1
        ;;
    esac
  fi
done

if [ "$FOUND_BINARIES" -eq 0 ]; then
  echo "No obsyncgit binaries were found in the archive" >&2
  exit 1
fi

OBSYNCHGIT_BIN="$INSTALL_DIR/obsyncgit"
if [ -x "$OBSYNCHGIT_BIN" ]; then
  INSTALLED_VERSION="$("$OBSYNCHGIT_BIN" --version 2>/dev/null || true)"
  [ -n "$INSTALLED_VERSION" ] && echo "$INSTALLED_VERSION"
fi

echo "Add $INSTALL_DIR to your PATH if it is not already there."

create_systemd_unit() {
  service_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
  mkdir -p "$service_dir"
  unit_path="$service_dir/obsyncgit.service"
  cat >"$unit_path" <<UNIT
[Unit]
Description=ObsyncGit daemon
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=$OBSYNCHGIT_BIN run
Restart=on-failure
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
UNIT

  if command -v systemctl >/dev/null 2>&1; then
    echo "Enabling obsyncgit systemd user service"
    systemctl --user daemon-reload || true
    systemctl --user enable obsyncgit.service || true
    systemctl --user restart obsyncgit.service || true
  else
    echo "systemctl not available; created unit at $unit_path"
  fi
}

create_launch_agent() {
  plist_dir="$HOME/Library/LaunchAgents"
  mkdir -p "$plist_dir"
  plist_path="$plist_dir/dev.obsyncgit.daemon.plist"
  cat >"$plist_path" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>dev.obsyncgit.daemon</string>
    <key>ProgramArguments</key>
    <array>
      <string>$OBSYNCHGIT_BIN</string>
      <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$HOME/Library/Logs/obsyncgit.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/Library/Logs/obsyncgit.err.log</string>
  </dict>
</plist>
PLIST

  if command -v launchctl >/dev/null 2>&1; then
    launchctl unload "$plist_path" >/dev/null 2>&1 || true
    launchctl load "$plist_path" || true
    echo "Configured LaunchAgent dev.obsyncgit.daemon"
  else
    echo "launchctl not available; created LaunchAgent at $plist_path"
  fi
}

case "$OS_ID" in
  linux)
    install_linux_runtime_deps
    create_systemd_unit
    ;;
  macos)
    create_launch_agent
    ;;
esac

cat <<"NOTE"

Autostart has been configured. You can manage it via systemd/launchctl or the obsyncgit-gui helper if installed.
NOTE
