#!/usr/bin/env bash
# lore installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/<user>/lore/main/install.sh | bash
#   ./install.sh   (from a local clone)
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO_RAW="https://raw.githubusercontent.com/<user>/lore/main"

die()  { echo "✗ $*" >&2; exit 1; }
ok()   { echo "✓ $*"; }
warn() { echo "⚠  $*"; }

install_binary() {
  mkdir -p "$INSTALL_DIR"

  # Running from a clone: lore is next to this script
  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || true)"

  if [[ -n "$script_dir" && -f "$script_dir/lore" ]]; then
    cp "$script_dir/lore" "$INSTALL_DIR/lore"
  else
    # Running via curl | bash: download from GitHub
    command -v curl > /dev/null 2>&1 || die "curl is required"
    curl -fsSL "$REPO_RAW/lore" -o "$INSTALL_DIR/lore"
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
