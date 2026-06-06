# lore

**L**ayered **O**rchestration for **R**ules and **E**xtensions

Universal skill and behavior manager for AI agents. Clone a repo, run one command, skills are live. No package registry, no vendor lock-in — just symlinks.

---

## The problem

`npx skills add` chokes on heavy repos. Skill configs scatter across `~/.claude/`, `~/.cursor/`, and wherever else, so you either duplicate installs or live with inconsistency. There's no standard place for agent rules and no clean way to compose behaviors from multiple sources.

lore fixes this with a single `~/.agents/` directory. All tools point there.

---

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/<user>/lore/main/install.sh | bash
```

Or from a clone:

```bash
git clone https://github.com/<user>/lore
cd lore && ./install.sh
```

Then bootstrap:

```bash
lore init
```

---

## Quick start

```bash
# Clone a skills repo and install from it
git clone https://github.com/someone/ai-skills ~/repos/ai-skills

cd ~/repos/ai-skills/skills
lore install coding-style type-conventions

cd ~/repos/ai-skills/behaviors
lore behavior add strict-review

lore list
```

---

## How it works

```
~/.agents/
├── AGENTS.md          ← universal agent config (managed by lore)
├── skills/            ← symlinks → skill dirs in your repos
└── behaviors/         ← symlinks → behavior dirs in your repos

~/.claude/
├── CLAUDE.md          ← @~/.agents/AGENTS.md  (written by lore init)
└── skills/            ← symlink → ~/.agents/skills/
```

Skills and behaviors live in their source repos. lore creates symlinks pointing to them. Installed = symlink exists. Uninstalled = symlink removed. The source is never touched.

When you run `lore behavior add`, the behavior's entry `.md` file gets appended to `~/.agents/AGENTS.md` as a Claude `@import`. Claude loads it on every session.

---

## Commands

| Command | Description |
|---|---|
| `lore init` | Bootstrap `~/.agents/` and wire Claude |
| `lore install <skill> [...]` | Install skill(s) from current directory |
| `lore remove <skill> [...]` | Uninstall skill(s) |
| `lore behavior add <name> [...]` | Install behavior(s) + update `AGENTS.md` |
| `lore behavior remove <name> [...]` | Remove behavior(s) |
| `lore list` | Show all installed skills and behaviors |
| `lore man` | Full manual |

Full reference: [docs/reference.md](docs/reference.md)

---

## Migrating from an existing Claude setup

`lore init` detects an existing `~/.claude/CLAUDE.md` and handles it automatically:

- Your old instructions are moved to `~/.agents/behaviors/from-claude/RULES.md`
- Existing skills in `~/.claude/skills/` are moved to `~/.agents/skills/`
- `~/.claude/CLAUDE.md` is replaced with a single `@import`

Nothing is lost. lore tells you exactly where everything went.

---

## Environment

`AGENTS_DIR` overrides the base directory (default: `~/.agents`). Useful for testing:

```bash
AGENTS_DIR=/tmp/test-agents lore init
```

---

## Roadmap

- [ ] Multi-account Claude support (`~/.claude-<account>/` directories)
- [ ] `lore update` — re-link skills after a repo has moved
- [ ] Additional tool integrations (Cursor, Windsurf, Zed)

---

## License

MIT
