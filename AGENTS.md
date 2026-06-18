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
│   ├── config.rs
│   ├── wire.rs
│   ├── symlink.rs
│   ├── agents_md.rs
│   └── commands/     ← includes accounts.rs
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

## Release process

`release-please` watches `main` (stable) and `dev` (beta pre-release) and parses
the **squash-merge commit subject** of every merge — not the individual commits
inside the PR. That subject must be a conventional commit (`feat:`, `fix:`,
`chore:`, `refactor:`, etc.) or release-please silently finds nothing to release.

- When merging a PR into `main` or `dev`, set the squash-merge commit title to a
  conventional commit message — GitHub defaults to the PR title, so title PRs
  accordingly.
- Tags are plain `v<version>` (`include-component-in-tag: false` in
  `release-please-config.json`) — this must keep matching the `v*` trigger in
  `.github/workflows/release.yml`.
- To pin a specific version on a release PR (e.g. bootstrapping the first
  release), add a `Release-As: X.Y.Z` footer to the triggering commit.

## Architecture

Everything reduces to three operations:

1. **Symlink management** — create/remove links in `~/.agents/skills/` or `~/.agents/behaviors/`
2. **AGENTS.md edits** — `AgentsMd` struct parses, mutates, and re-serializes the file
3. **Claude wiring** — write `@AGENTS_MD` to `~/.claude/CLAUDE.md`, symlink `~/.claude/skills`

## Coding conventions

- Rust. Deps: `clap` (derive), `anyhow`, `dirs`, `serde` (derive), `toml`. Dev: `assert_cmd`, `tempfile`, `predicates`.
- All commands return `anyhow::Result<()>`. Dispatch in `lib.rs::run()`.
- Command functions in `src/commands/`. Utils in `src/`.
- Output: `ok()` for success, `warn()` for non-fatal issues, `note()` for indented sub-info.
- Config lives at `~/.config/lore/lore.toml` (override path: `LORE_CONF` env var — the
  only env var lore reads). Holds `agents_dir` and the `[accounts]` registry.

## Key invariants — do not break

1. `lore init` is idempotent — safe to re-run at any time
2. Uninstalling never modifies source repos — only removes symlinks
3. `behavior add` is idempotent — checks AGENTS.md before appending
4. AGENTS.md block format is exactly two lines: `<!-- name -->` then `@/absolute/path`
5. `init` case 2 must never lose existing CLAUDE.md content — always migrates to `from-claude`
6. Broken symlinks must appear in `lore list` (use `/*` glob, not `*/`)
7. Config is always read via `LoreConfig::load_or_default` — never hand-roll a TOML read
8. `Paths` is agents-only — no `claude_dir` field; per-account Claude dirs are resolved
   in `commands/init.rs`/`commands/accounts.rs` from the config's accounts registry
9. `accounts remove` only ever touches the registry — never disk (`~/.claude-<name>/`)

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
echo 'agents_dir = "/tmp/lore-test/agents"' > /tmp/lore-test.toml
export LORE_CONF=/tmp/lore-test.toml
lore init

mkdir -p /tmp/fake/skills/my-skill && touch /tmp/fake/skills/my-skill/SKILL.md
cd /tmp/fake/skills && lore install my-skill

mkdir -p /tmp/fake/behaviors/my-rules && touch /tmp/fake/behaviors/my-rules/RULES.md
cd /tmp/fake/behaviors && lore behavior add my-rules

lore list
cat /tmp/lore-test/agents/AGENTS.md
```

## Planned work

- **`lore update`**: re-link skills after a repo has moved on disk.
- **Additional tool integrations**: Cursor, Windsurf, Zed — each needs its own wiring
  in `commands/init.rs`, modeled after the Claude integration.
