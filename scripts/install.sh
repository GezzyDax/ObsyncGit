#!/usr/bin/env sh
set -eu

REPO="GezzyDax/ObsyncGit"
PROJECT="obsyncgit"
DEFAULT_INSTALL_DIR="/usr/local/bin"
INSTALL_DIR="${OBSYNCGIT_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
VERSION="${OBSYNCGIT_VERSION:-latest}"

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
BINARY_PATH="$TMPDIR/$PROJECT"

if [ ! -x "$BINARY_PATH" ]; then
  echo "Failed to extract executable" >&2
  exit 1
fi

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

DEST_PATH="$INSTALL_DIR/$PROJECT"

if [ -w "$INSTALL_DIR" ]; then
  install -m 755 "$BINARY_PATH" "$DEST_PATH"
else
  if command -v sudo >/dev/null 2>&1; then
    echo "Using sudo to install into $INSTALL_DIR"
    sudo install -m 755 "$BINARY_PATH" "$DEST_PATH"
  else
    echo "Insufficient permissions to write to $INSTALL_DIR" >&2
    exit 1
  fi
fi

INSTALLED_VERSION="$("$DEST_PATH" --version 2>/dev/null || true)"

echo "Installed $PROJECT to $DEST_PATH"
[ -n "$INSTALLED_VERSION" ] && echo "$INSTALLED_VERSION"

echo "Add $INSTALL_DIR to your PATH if it is not already there."
