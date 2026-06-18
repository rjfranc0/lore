# lore — development context

## What this is

lore is a Rust CLI that manages AI agent skills and behaviors via symlinks.
One universal config dir at `~/.agents/`, Claude wired via `~/.claude/`.

## Repo structure

```
lore/
├── src/              ← Rust source
│   ├── lib.rs
│   ├── main.rs
│   ├── cli.rs
│   ├── output.rs
│   ├── paths.rs
│   ├── symlink.rs
│   ├── agents_md.rs
│   └── commands/
├── tests/integration/
├── .githooks/        ← local git hooks (opt-in)
├── .github/workflows/
├── install.sh        ← installs lore to ~/.local/bin
├── Cargo.toml
├── README.md
├── AGENTS.md         ← you are here
├── CLAUDE.md         ← @AGENTS.md
└── docs/
    └── reference.md  ← full command and format reference
```

## Local setup

After cloning, activate the pre-push hook to run `cargo test + clippy` before each push:

```bash
git config core.hooksPath .githooks
```

This is opt-in — the hook is not forced on contributors.

## Architecture

Everything reduces to three operations:

1. **Symlink management** — create/remove links in `~/.agents/skills/` or `~/.agents/behaviors/`
2. **AGENTS.md edits** — `AgentsMd` struct parses, mutates, and re-serializes the file
3. **Claude wiring** — write `@AGENTS_MD` to `~/.claude/CLAUDE.md`, symlink `~/.claude/skills`

## Coding conventions

- Rust. Deps: `clap` (derive), `anyhow`, `dirs`. Dev: `assert_cmd`, `tempfile`, `predicates`.
- All commands return `anyhow::Result<()>`. Dispatch in `lib.rs::run()`.
- Command functions in `src/commands/`. Utils in `src/`.
- Output: `ok()` for success, `warn()` for non-fatal issues, `note()` for indented sub-info.

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

The comment is the lookup key for removal (`AgentsMd::remove_by_name`).
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
  in `commands/init.rs`, modeled after the Claude integration.
