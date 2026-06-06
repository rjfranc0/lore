#!/usr/bin/env bats
# Run: bats test/install.bats
# Install bats-core first: brew install bats-core

setup() {
  export HOME
  HOME="$(mktemp -d)"
  export INSTALL_DIR="$HOME/.local/bin"
  INSTALL_SH="$BATS_TEST_DIRNAME/../install.sh"
  LORE_SCRIPT="$BATS_TEST_DIRNAME/../lore"
}

teardown() {
  rm -rf "$HOME"
}

# ── helpers ───────────────────────────────────────────────────────────────────

fake_curl_ok() {  # fake_curl_ok <bin_dir> — writes a dummy executable to -o target
  mkdir -p "$1"
  cat > "$1/curl" << 'CURL'
#!/usr/bin/env bash
outfile=""
while [[ $# -gt 0 ]]; do
  case "$1" in -o) shift; outfile="$1" ;; esac
  shift
done
printf '#!/usr/bin/env bash\necho "fake lore"\n' > "$outfile"
CURL
  chmod +x "$1/curl"
}

fake_curl_fail() {  # fake_curl_fail <bin_dir> — curl exits 6 (connection error)
  mkdir -p "$1"
  printf '#!/usr/bin/env bash\nexit 6\n' > "$1/curl"
  chmod +x "$1/curl"
}

# ── local clone ───────────────────────────────────────────────────────────────

@test "local: copies lore to INSTALL_DIR" {
  run bash "$INSTALL_SH"
  [ "$status" -eq 0 ]
  [ -f "$INSTALL_DIR/lore" ]
}

@test "local: installed binary is executable" {
  bash "$INSTALL_SH"
  [ -x "$INSTALL_DIR/lore" ]
}

@test "local: creates INSTALL_DIR if absent" {
  export INSTALL_DIR="$HOME/deep/nested/bin"
  run bash "$INSTALL_SH"
  [ "$status" -eq 0 ]
  [ -d "$INSTALL_DIR" ]
  [ -f "$INSTALL_DIR/lore" ]
}

@test "local: output confirms install path" {
  run bash "$INSTALL_SH"
  [ "$status" -eq 0 ]
  [[ "$output" == *"✓ Installed lore"* ]]
  [[ "$output" == *"$INSTALL_DIR/lore"* ]]
}

@test "local: installed binary matches source" {
  bash "$INSTALL_SH"
  diff "$LORE_SCRIPT" "$INSTALL_DIR/lore"
}

# ── PATH check ────────────────────────────────────────────────────────────────

@test "path check: warns when INSTALL_DIR not in PATH" {
  run env PATH="/usr/bin:/bin" bash "$INSTALL_SH"
  [ "$status" -eq 0 ]
  [[ "$output" == *"not in your PATH"* ]]
}

@test "path check: no warning when INSTALL_DIR is in PATH" {
  run env PATH="$INSTALL_DIR:/usr/bin:/bin" bash "$INSTALL_SH"
  [ "$status" -eq 0 ]
  [[ "$output" != *"not in your PATH"* ]]
}

# ── download path ─────────────────────────────────────────────────────────────

@test "download: installs via curl when no local lore beside script" {
  # Copy install.sh to HOME (no lore next to it) → curl branch taken
  cp "$INSTALL_SH" "$HOME/install.sh"
  local fakebin="$HOME/fake-bin"
  fake_curl_ok "$fakebin"
  run env PATH="$fakebin:/usr/bin:/bin" bash "$HOME/install.sh"
  [ "$status" -eq 0 ]
  [ -f "$INSTALL_DIR/lore" ]
  [ -x "$INSTALL_DIR/lore" ]
}

@test "download: friendly error with URL hint when curl fails" {
  cp "$INSTALL_SH" "$HOME/install.sh"
  local fakebin="$HOME/fake-bin"
  fake_curl_fail "$fakebin"
  run env PATH="$fakebin:/usr/bin:/bin" bash "$HOME/install.sh"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Download failed"* ]]
  [[ "$output" == *"check URL"* ]]
}

# ── BASH_SOURCE fix ───────────────────────────────────────────────────────────

@test "BASH_SOURCE: piped execution does not crash under set -u" {
  # bash < file leaves BASH_SOURCE[0] empty — old code hit 'unbound variable'
  # Run from HOME (no lore file there) so the curl branch is taken
  local fakebin="$HOME/fake-bin"
  fake_curl_ok "$fakebin"
  run bash -c "cd '$HOME' && env PATH='$fakebin:/usr/bin:/bin' INSTALL_DIR='$INSTALL_DIR' bash < '$INSTALL_SH'"
  [ "$status" -eq 0 ]
  [ -f "$INSTALL_DIR/lore" ]
}
