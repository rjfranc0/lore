# lore

**L**ayered **O**rchestration for **R**ules and **E**xtensions

A small tool for managing the skills and behaviors your AI agents use. Clone a repo, run one command, and the skills are live. There's no package registry behind it and nothing to lock you in — under the hood it's just symlinks.

## Why it exists

If you've tried `npx skills add` on a heavy repo, you know it tends to choke. And once you're running more than one agent, your config ends up scattered: a bit in `~/.claude/`, a bit in `~/.cursor/`, more somewhere else. You either install everything twice or quietly let the setups drift apart. There's no agreed-on home for agent rules, and no clean way to mix behaviors coming from different repos.

lore's answer is boring on purpose: one directory, `~/.agents/`, that every tool points at.

## Install

Grab the latest stable binary:

```bash
curl -fsSL https://raw.githubusercontent.com/rjfranc0/lore/main/install.sh | bash
```

Or clone and let the installer build from source (requires `cargo`):

```bash
git clone https://github.com/rjfranc0/lore
cd lore && ./install.sh
```

To install the latest pre-release from the dev channel:

```bash
curl -fsSL https://raw.githubusercontent.com/rjfranc0/lore/main/install.sh | bash -s -- --dev
```

Either way, bootstrap once when you're done:

```bash
lore init
```

## A quick taste

Say you've cloned a repo full of skills and behaviors. You install them from wherever they live:

```bash
git clone https://github.com/someone/ai-skills ~/repos/ai-skills

cd ~/repos/ai-skills/skills
lore install coding-style type-conventions

cd ~/repos/ai-skills/behaviors
lore behavior add strict-review

lore list
```

That's the whole loop: `cd` into a repo, install what you want, check it with `list`.

## How it actually works

After `lore init`, your layout looks like this:

```
~/.agents/
├── AGENTS.md          ← universal agent config (managed by lore)
├── skills/            ← symlinks → skill dirs in your repos
└── behaviors/         ← symlinks → behavior dirs in your repos

~/.claude/
├── LORE.md            ← @~/.agents/AGENTS.md  (written by lore init)
├── CLAUDE.md          ← @~/.claude/LORE.md  (lore only ensures this one line)
└── skills/            ← symlink → ~/.agents/skills/
```

The important part: your skills and behaviors never leave their source repos. lore just creates symlinks that point at them. A skill is "installed" when its symlink exists and "uninstalled" when you remove it — the original files are never moved or modified.

Behaviors work the same way, with one extra step. When you run `lore behavior add`, lore finds the behavior's entry `.md` file and appends it to `~/.agents/AGENTS.md` as a Claude `@import`. From then on, Claude loads it at the start of every session.

## Commands

| Command | What it does |
|---|---|
| `lore init` | Bootstrap `~/.agents/` and wire up Claude |
| `lore install <skill> [...]` | Install skill(s) from the current directory |
| `lore remove <skill> [...]` | Uninstall skill(s) |
| `lore behavior add <name> [...]` | Install behavior(s) and update `AGENTS.md` |
| `lore behavior remove <name> [...]` | Remove behavior(s) |
| `lore sync` | Reconcile `AGENTS.md` from disk |
| `lore list` | Show everything that's installed |
| `lore version` | Print the version |
| `lore help` | Full manual |

The full write-up of every command and file format lives in [docs/index.md](docs/index.md).

## Already have a Claude setup?

You don't have to clean anything up first. `lore init` notices existing content in `~/.claude/CLAUDE.md` and migrates it for you, surgically:

- A copy of your old instructions moves to `~/.agents/behaviors/from-claude/RULES.md`
- Any skills sitting in `~/.claude/skills/` move into `~/.agents/skills/`
- `~/.claude/CLAUDE.md` keeps its original content exactly as it was — lore only appends a `@~/.claude/LORE.md` import line underneath it

Nothing gets thrown away or overwritten, and lore prints exactly where each piece landed, plus any `@import` lines left over from other tools so you know they're still there, untouched. If a skill name would collide with one you already have, lore stops before changing anything and tells you what to resolve — better a clear halt than a half-migrated mess. And if you ever delete `AGENTS.md` by accident, re-running `lore init` rebuilds it from the behaviors still on disk.

## Pointing lore somewhere else

Two environment variables let you redirect where lore reads and writes. Set both and you can exercise the whole tool without going near your real config — handy for testing, or for running more than one Claude profile:

```bash
AGENTS_DIR=/tmp/test/agents CLAUDE_DIR=/tmp/test/claude lore init
```

`AGENTS_DIR` defaults to `~/.agents` and moves every derived path with it. `CLAUDE_DIR` defaults to `~/.claude`.

## Tests

The test suite uses `cargo test` — no extra tooling required:

```bash
cargo test
```

Tests run against throwaway temp directories, so they never touch your real setup.

## Roadmap

- [ ] Multi-account Claude support (`~/.claude-<account>/` directories)
- [ ] `lore update` — re-link skills after a repo moves on disk
- [ ] Integrations for other tools (Cursor, Windsurf, Zed)

## License

MIT
