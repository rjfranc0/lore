#!/usr/bin/env bash
# lore installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/rjfranc0/lore/main/install.sh | bash
#   ./install.sh          (from a local clone — builds from source if cargo is available)
#   ./install.sh --dev    (install latest pre-release from the dev channel)
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO="rjfranc0/lore"
CHANNEL="stable"

die()  { echo "✗ $*" >&2; exit 1; }
ok()   { echo "✓ $*"; }
warn() { echo "⚠  $*"; }

for arg in "$@"; do
  case "$arg" in
    --dev) CHANNEL="dev" ;;
    *) die "Unknown argument: $arg" ;;
  esac
done

detect_platform() {
  local os arch
  case "$(uname -s)" in
    Linux)  os="linux"  ;;
    Darwin) os="macos"  ;;
    *) die "Unsupported OS: $(uname -s)" ;;
  esac
  case "$(uname -m)" in
    x86_64)        arch="x86_64"  ;;
    aarch64|arm64) arch="aarch64" ;;
    *) die "Unsupported arch: $(uname -m)" ;;
  esac
  echo "lore-${os}-${arch}"
}

download_stable() {
  local artifact="$1"
  local url="https://github.com/${REPO}/releases/latest/download/${artifact}"
  echo "Downloading $artifact from stable channel…"
  curl -fsSL "$url" -o "$INSTALL_DIR/lore" \
    || die "Download failed — check URL: $url"
}

download_dev() {
  local artifact="$1"
  echo "Querying dev channel for latest pre-release…"
  local url
  url=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
    | grep -m1 "browser_download_url.*${artifact}" \
    | cut -d'"' -f4)
  [[ -n "$url" ]] || die "No pre-release binary found for ${artifact}"
  echo "Downloading $artifact from dev channel…"
  curl -fsSL "$url" -o "$INSTALL_DIR/lore" \
    || die "Download failed — check URL: $url"
}

install_binary() {
  mkdir -p "$INSTALL_DIR"
  command -v curl > /dev/null 2>&1 || die "curl is required"

  local artifact
  artifact="$(detect_platform)"

  if [[ "$CHANNEL" == "dev" ]]; then
    download_dev "$artifact"
  else
    # Stable channel: prefer local clone over remote download
    local script_dir=""
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-}")" 2>/dev/null && pwd || true)"

    if [[ -n "$script_dir" && -f "$script_dir/target/release/lore" ]]; then
      cp "$script_dir/target/release/lore" "$INSTALL_DIR/lore"
      echo "Installed from local build."
    elif [[ -n "$script_dir" ]] && command -v cargo > /dev/null 2>&1 && [[ -f "$script_dir/Cargo.toml" ]]; then
      echo "Building from source…"
      (cd "$script_dir" && cargo build --release --quiet)
      cp "$script_dir/target/release/lore" "$INSTALL_DIR/lore"
      echo "Built and installed from source."
    else
      download_stable "$artifact"
    fi
  fi

  chmod +x "$INSTALL_DIR/lore"
  ok "Installed lore → $INSTALL_DIR/lore"
}

check_path() {
  if ! echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
    warn "$INSTALL_DIR is not in your PATH"
    echo ""
    echo "  Add this to your shell config (~/.zshrc or ~/.bashrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "  Then: source ~/.zshrc"
    echo ""
  fi
}

install_binary
check_path
echo "Run 'lore init' to get started."
