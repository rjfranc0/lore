# lore — development context

## Read the docs first

`docs/` is the source of truth for what lore does and how it's built —
start at [docs/index.md](docs/index.md). Docs are the cognitive model of
the system; code is the implementation detail. This file is a thin,
doc-external supplement (setup, conventions, pointers) — never a second
copy of anything `docs/` already explains, since a duplicate is one more
place to go stale.

**Before touching command behavior, invariants, symlink/config internals,
or the release pipeline, read the matching file under
`docs/functional/`, `docs/implementation/`, or `docs/infra/` first** —
don't rely on your own reading of the code or on memory of a past session.
Each `implementation/*.md` file's "What breaks if this is touched" section
is the fastest way to check whether a change is safe. If a doc file is
missing or doesn't cover what you need, that's a gap to flag (or fix via
the `corpus` skill) — not license to skip reading and guess instead.

## What this is

lore is a Rust CLI that manages AI agent skills and behaviors via
symlinks. See [docs/index.md](docs/index.md) for the full model.

## Repo structure

```
lore/
├── src/              ← Rust source (see docs/implementation/index.md)
│   ├── lib.rs, main.rs, cli.rs        ← entry point + dispatch
│   ├── config.rs, paths.rs, wire.rs   ← config/account/wiring layer
│   ├── symlink.rs, output.rs          ← low-level primitives
│   ├── agents_md.rs                   ← AGENTS.md/LORE.md parser
│   └── commands/                      ← one module per subcommand
├── tests/integration/
├── .githooks/        ← local git hooks (opt-in)
├── .github/workflows/
├── install.sh        ← installs lore to ~/.local/bin
├── docs/             ← corpus-generated reference — start here
├── AGENTS.md         ← you are here
└── CLAUDE.md         ← @AGENTS.md
```

## Local setup

```bash
git config core.hooksPath .githooks
```

Activates the pre-push hook (`cargo test` + `cargo clippy`). See
[docs/infra/release-and-distribution.md](docs/infra/release-and-distribution.md)
for why it's opt-in and what CI already gates on every push.

## Coding conventions

This section is intentionally a short pointer list, not the full
corpus convention template (no separate Error Handling / Async Conventions
/ Commit Format subsections) — lore is a small, synchronous CLI with one
error type (`anyhow::Result`) and no async code, so those categories would
be empty or redundant with the point below. Extend this list, not the
template shape, if that ever stops being true.

- Rust. Deps: `clap` (derive), `anyhow`, `dirs`, `serde` (derive), `toml`.
  Dev: `assert_cmd`, `tempfile`, `predicates`.
- All commands return `anyhow::Result<()>`. Dispatch happens in
  `lib.rs::run()` — see [docs/implementation/cli.md](docs/implementation/cli.md).
- Command functions live in `src/commands/`, one module per subcommand.
  Shared utilities live directly in `src/`.
- Output helpers: `ok()` for success, `warn()` for non-fatal issues,
  `note()` for indented sub-info.
- `AGENTS.md`/`LORE.md`'s block format and every config/wiring contract
  (config path resolution, account registry shape, etc.) are documented in
  [docs/functional/agent-config/index.md](docs/functional/agent-config/index.md)
  and [docs/implementation/accounts/index.md](docs/implementation/accounts/index.md)
  — read those rather than re-deriving the format from `agents_md.rs`.

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

- **Additional tool integrations**: Cursor, Windsurf, Zed — each needs its
  own wiring in `commands/init.rs`, modeled after the Claude integration.
