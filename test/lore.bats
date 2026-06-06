#!/usr/bin/env bats
# Run: bats test/lore.bats
# Install bats-core first: brew install bats-core

setup() {
  export HOME
  HOME="$(mktemp -d)"
  export CLAUDE_DIR="$HOME/.claude"
  export AGENTS_DIR="$HOME/.agents"
  mkdir -p "$CLAUDE_DIR"
  LORE="$BATS_TEST_DIRNAME/../lore"
}

teardown() {
  rm -rf "$HOME"
}

# ── helpers ───────────────────────────────────────────────────────────────────

make_skill() {   # make_skill <base_dir> <name>
  mkdir -p "$1/$2"; touch "$1/$2/SKILL.md"
}

make_behavior() {  # make_behavior <base_dir> <name> [entry=RULES.md]
  local entry="${3:-RULES.md}"
  mkdir -p "$1/$2"; printf 'rules\n' > "$1/$2/$entry"
}

# ── init ──────────────────────────────────────────────────────────────────────

@test "init: creates expected structure (case 1)" {
  run "$LORE" init
  [ "$status" -eq 0 ]
  [ -d "$AGENTS_DIR/skills" ]
  [ -d "$AGENTS_DIR/behaviors" ]
  [ -f "$AGENTS_DIR/AGENTS.md" ]
  [ -f "$CLAUDE_DIR/CLAUDE.md" ]
  [ -L "$CLAUDE_DIR/skills" ]
  grep -qF "@$AGENTS_DIR/AGENTS.md" "$CLAUDE_DIR/CLAUDE.md"
}

@test "init: is idempotent" {
  "$LORE" init
  run "$LORE" init
  [ "$status" -eq 0 ]
  [ -L "$CLAUDE_DIR/skills" ]
}

@test "init: migrates existing CLAUDE.md content (case 2)" {
  printf '# old rules\nbe nice\n' > "$CLAUDE_DIR/CLAUDE.md"
  run "$LORE" init
  [ "$status" -eq 0 ]
  [ -f "$AGENTS_DIR/behaviors/from-claude/RULES.md" ]
  grep -qF "old rules" "$AGENTS_DIR/behaviors/from-claude/RULES.md"
  grep -qF "@$AGENTS_DIR/AGENTS.md" "$CLAUDE_DIR/CLAUDE.md"
  grep -qF "from-claude" "$AGENTS_DIR/AGENTS.md"
}

@test "init: safe-fail on skill collision — CLAUDE.md not written" {
  # Put a real skills dir in CLAUDE_DIR with an item that also exists in AGENTS_DIR/skills
  mkdir -p "$CLAUDE_DIR/skills/dup" "$AGENTS_DIR/skills/dup"
  touch "$CLAUDE_DIR/skills/dup/SKILL.md" "$AGENTS_DIR/skills/dup/SKILL.md"
  # AGENTS.md must exist so we don't go into migration path
  mkdir -p "$AGENTS_DIR/behaviors"
  printf '<!-- managed by lore -->\n' > "$AGENTS_DIR/AGENTS.md"
  # Overwrite CLAUDE.md with the pointer so init skips migration
  printf '@%s/AGENTS.md\n' "$AGENTS_DIR" > "$CLAUDE_DIR/CLAUDE.md"

  run "$LORE" init
  [ "$status" -ne 0 ]
  # CLAUDE.md must still be the old pointer, not overwritten with partial state
  grep -qF "@$AGENTS_DIR/AGENTS.md" "$CLAUDE_DIR/CLAUDE.md"
  # skills symlink must not have been created
  [ ! -L "$CLAUDE_DIR/skills" ]
}

@test "init: recovery — re-registers existing behaviors when AGENTS.md deleted" {
  "$LORE" init
  # Add a behavior
  local bdir="$HOME/src-behaviors/my-rules"
  make_behavior "$HOME/src-behaviors" "my-rules"
  cd "$HOME/src-behaviors" && "$LORE" behavior add my-rules
  grep -qF "<!-- my-rules -->" "$AGENTS_DIR/AGENTS.md"

  # Simulate user deleting AGENTS.md
  rm "$AGENTS_DIR/AGENTS.md"

  run "$LORE" init
  [ "$status" -eq 0 ]
  grep -qF "<!-- my-rules -->" "$AGENTS_DIR/AGENTS.md"
}

# ── install ───────────────────────────────────────────────────────────────────

@test "install: creates symlink to skill" {
  "$LORE" init
  make_skill "$HOME/src" "my-skill"
  cd "$HOME/src" && run "$LORE" install my-skill
  [ "$status" -eq 0 ]
  [ -L "$AGENTS_DIR/skills/my-skill" ]
  [ "$(readlink "$AGENTS_DIR/skills/my-skill")" = "$HOME/src/my-skill" ]
}

@test "install: warns on duplicate, does not overwrite" {
  "$LORE" init
  make_skill "$HOME/src" "dup"
  cd "$HOME/src" && "$LORE" install dup
  run bash -c "cd '$HOME/src' && '$LORE' install dup"
  [ "$status" -eq 0 ]
  [[ "$output" == *"already installed"* ]]
}

@test "install: exits 1 for missing skill" {
  "$LORE" init
  cd "$HOME"
  run "$LORE" install does-not-exist
  [ "$status" -ne 0 ]
}

@test "install: normalizes trailing slash from tab-completion" {
  "$LORE" init
  make_skill "$HOME/src" "tabskill"
  cd "$HOME/src" && run "$LORE" install tabskill/
  [ "$status" -eq 0 ]
  [ -L "$AGENTS_DIR/skills/tabskill" ]
}

# ── remove ────────────────────────────────────────────────────────────────────

@test "remove: removes symlink, source dir untouched" {
  "$LORE" init
  make_skill "$HOME/src" "gone"
  cd "$HOME/src" && "$LORE" install gone
  run "$LORE" remove gone
  [ "$status" -eq 0 ]
  [ ! -L "$AGENTS_DIR/skills/gone" ]
  [ -d "$HOME/src/gone" ]
}

@test "remove: normalizes trailing slash" {
  "$LORE" init
  make_skill "$HOME/src" "tabremove"
  cd "$HOME/src" && "$LORE" install tabremove
  run "$LORE" remove tabremove/
  [ "$status" -eq 0 ]
  [ ! -L "$AGENTS_DIR/skills/tabremove" ]
}

@test "remove: warns but exits 0 when skill not installed" {
  "$LORE" init
  run "$LORE" remove nonexistent
  [ "$status" -eq 0 ]
  [[ "$output" == *"not installed"* ]]
}

# ── behavior add ──────────────────────────────────────────────────────────────

@test "behavior add: creates symlink and AGENTS.md block" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "my-rules"
  cd "$HOME/bsrc" && run "$LORE" behavior add my-rules
  [ "$status" -eq 0 ]
  [ -L "$AGENTS_DIR/behaviors/my-rules" ]
  grep -qF "<!-- my-rules -->" "$AGENTS_DIR/AGENTS.md"
  grep -qF "@$AGENTS_DIR/behaviors/my-rules/RULES.md" "$AGENTS_DIR/AGENTS.md"
}

@test "behavior add: RULES.md takes priority over README.md" {
  "$LORE" init
  mkdir -p "$HOME/bsrc/mixed"
  printf 'readme\n' > "$HOME/bsrc/mixed/README.md"
  printf 'rules\n'  > "$HOME/bsrc/mixed/RULES.md"
  cd "$HOME/bsrc" && "$LORE" behavior add mixed
  grep -qF "RULES.md" "$AGENTS_DIR/AGENTS.md"
  ! grep -qF "README.md" "$AGENTS_DIR/AGENTS.md"
}

@test "behavior add: is idempotent" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "once"
  cd "$HOME/bsrc" && "$LORE" behavior add once
  run bash -c "cd '$HOME/bsrc' && '$LORE' behavior add once"
  [ "$status" -eq 0 ]
  local count
  count=$(grep -c "<!-- once -->" "$AGENTS_DIR/AGENTS.md" || true)
  [ "$count" -eq 1 ]
}

@test "behavior add: normalizes trailing slash" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "tabbed"
  cd "$HOME/bsrc" && run "$LORE" behavior add tabbed/
  [ "$status" -eq 0 ]
  [ -L "$AGENTS_DIR/behaviors/tabbed" ]
}

@test "behavior add: fails with helpful message before init" {
  make_behavior "$HOME/bsrc" "early"
  cd "$HOME/bsrc" && run "$LORE" behavior add early
  [ "$status" -ne 0 ]
  [[ "$output" == *"lore init"* ]]
}

# ── behavior remove ───────────────────────────────────────────────────────────

@test "behavior remove: removes symlink and AGENTS.md block" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "bye"
  cd "$HOME/bsrc" && "$LORE" behavior add bye
  run "$LORE" behavior remove bye
  [ "$status" -eq 0 ]
  [ ! -L "$AGENTS_DIR/behaviors/bye" ]
  ! grep -qF "<!-- bye -->" "$AGENTS_DIR/AGENTS.md"
}

@test "behavior remove: exact match — regex-special name does not clobber sibling" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "a.c"
  make_behavior "$HOME/bsrc" "axc"
  cd "$HOME/bsrc" && "$LORE" behavior add a.c axc
  run "$LORE" behavior remove a.c
  [ "$status" -eq 0 ]
  ! grep -qF "<!-- a.c -->" "$AGENTS_DIR/AGENTS.md"
  grep -qF "<!-- axc -->" "$AGENTS_DIR/AGENTS.md"
  [ -L "$AGENTS_DIR/behaviors/axc" ]
}

@test "behavior remove: normalizes trailing slash" {
  "$LORE" init
  make_behavior "$HOME/bsrc" "trailme"
  cd "$HOME/bsrc" && "$LORE" behavior add trailme
  run "$LORE" behavior remove trailme/
  [ "$status" -eq 0 ]
  ! grep -qF "<!-- trailme -->" "$AGENTS_DIR/AGENTS.md"
}

# ── list ──────────────────────────────────────────────────────────────────────

@test "list: shows installed skill symlink" {
  "$LORE" init
  make_skill "$HOME/src" "visible"
  cd "$HOME/src" && "$LORE" install visible
  run "$LORE" list
  [[ "$output" == *"visible"* ]]
  [[ "$output" != *"✗ broken"* ]]
}

@test "list: flags broken skill symlink" {
  "$LORE" init
  mkdir -p "$HOME/gone-src/ghost"; touch "$HOME/gone-src/ghost/SKILL.md"
  cd "$HOME/gone-src" && "$LORE" install ghost
  rm -rf "$HOME/gone-src"
  run "$LORE" list
  [[ "$output" == *"ghost"* ]]
  [[ "$output" == *"✗ broken"* ]]
}

@test "list: shows migrated (real dir) skill as (migrated)" {
  "$LORE" init
  # Simulate a real dir in skills (as created by init migration)
  mkdir -p "$AGENTS_DIR/skills/migrated-skill"
  touch "$AGENTS_DIR/skills/migrated-skill/SKILL.md"
  run "$LORE" list
  [[ "$output" == *"migrated-skill"* ]]
  [[ "$output" == *"(migrated)"* ]]
}

@test "list: shows built-in behavior as (built-in)" {
  printf '# old\n' > "$CLAUDE_DIR/CLAUDE.md"
  "$LORE" init
  run "$LORE" list
  [[ "$output" == *"from-claude"* ]]
  [[ "$output" == *"(built-in)"* ]]
}

# ── version ───────────────────────────────────────────────────────────────────

@test "version: prints lore version string" {
  run "$LORE" version
  [ "$status" -eq 0 ]
  [[ "$output" == "lore "* ]]
}

# ── dispatch ──────────────────────────────────────────────────────────────────

@test "unknown subcommand exits 1" {
  run "$LORE" definitely-not-a-command
  [ "$status" -eq 1 ]
}

@test "help exits 0" {
  run "$LORE" help
  [ "$status" -eq 0 ]
}
