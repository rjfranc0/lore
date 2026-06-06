# lore — development context

## What this is

lore is a single bash script (~250 lines) that manages AI agent skills and behaviors
via symlinks. One universal config dir at `~/.agents/`, Claude wired via `~/.claude/`.

No external dependencies. No build step. No package manager.

## Repo structure

```
lore/
├── lore              ← the CLI (single file, this is the whole product)
├── install.sh        ← installs lore to ~/.local/bin
├── README.md
├── AGENTS.md         ← you are here
├── CLAUDE.md         ← @AGENTS.md
└── docs/
    └── reference.md  ← full command and format reference
```

## Architecture

Everything reduces to three operations:

1. **Symlink management** — create/remove links in `~/.agents/skills/` or `~/.agents/behaviors/`
2. **AGENTS.md edits** — append or delete `<!-- name -->\n@path` blocks
3. **Claude wiring** — write `@AGENTS_MD` to `~/.claude/CLAUDE.md`, symlink `~/.claude/skills`

## Coding conventions

- Bash only. POSIX-ish. Deps: `find`, `sed`, `ln`, `readlink` — all standard.
- `set -euo pipefail` at the top, always.
- `sedi()` for every `sed -i` call — handles BSD (macOS) vs GNU (Linux) flag difference.
- `shopt -s nullglob` / `shopt -u nullglob` around globs that might match nothing.
- Never use `((count++))` — `set -e` treats `((0))` as failure. Use `count=$((count + 1))`.
- `if [[ ... ]]; then` not `[[ ... ]] && command` — cleaner with `set -e`.
- Command functions: `cmd_*`. Utils: lowercase, no prefix.
- Output: `ok()` for success, `warn()` for non-fatal issues, `die()` to exit, `note()` for indented sub-info.

## Key invariants — do not break

1. `lore init` is idempotent — safe to re-run at any time
2. Uninstalling never modifies source repos — only removes symlinks
3. `behavior add` is idempotent — checks AGENTS.md before appending
4. AGENTS.md block format is exactly two lines: `<!-- name -->` then `@/absolute/path`
5. `init` case 2 must never lose existing CLAUDE.md content — always migrates to `from-claude`
6. Broken symlinks must appear in `lore list` (use `/*` glob, not `*/`)

## AGENTS.md block format

Every behavior entry is exactly this:

```
<!-- name -->
@/absolute/path/to/entry.md
```

The comment is the lookup key for removal (`sed /<!-- name -->/,+1d`).
The `@path` is what Claude imports. Always absolute paths.

## Testing without touching real config

```bash
AGENTS_DIR=/tmp/lore-test lore init

mkdir -p /tmp/fake/skills/my-skill && touch /tmp/fake/skills/my-skill/SKILL.md
cd /tmp/fake/skills && AGENTS_DIR=/tmp/lore-test lore install my-skill

mkdir -p /tmp/fake/behaviors/my-rules && touch /tmp/fake/behaviors/my-rules/RULES.md
cd /tmp/fake/behaviors && AGENTS_DIR=/tmp/lore-test lore behavior add my-rules

AGENTS_DIR=/tmp/lore-test lore list
cat /tmp/lore-test/AGENTS.md
```

## Planned work

- **Multi-account support**: Claude supports `~/.claude-<account>/` dirs. The design
  (one base `~/.agents/` + per-account overrides, or fully independent instances)
  is not finalized. This is a breaking surface — design carefully before touching init.
- **`lore update`**: re-link skills after a repo has moved on disk.
- **Additional tool integrations**: Cursor, Windsurf, Zed — each needs its own wiring
  in `cmd_init`, modeled after the Claude integration.
