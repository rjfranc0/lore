# lore reference

Full command and format reference for lore.

---

## Commands

### `lore init`

Bootstrap `~/.agents/` and wire Claude integration.

**Case 1 — empty install** (no existing `~/.claude/CLAUDE.md`):

- Creates `~/.agents/AGENTS.md`, `~/.agents/skills/`, `~/.agents/behaviors/`
- Writes `@~/.agents/AGENTS.md` into `~/.claude/CLAUDE.md`
- Symlinks `~/.claude/skills → ~/.agents/skills`

**Case 2 — existing config:**

- Reads `~/.claude/CLAUDE.md` content and writes it to `~/.agents/behaviors/from-claude/RULES.md`
- Creates `~/.agents/AGENTS.md` with `from-claude` as the first behavior entry
- Moves any real skills from `~/.claude/skills/` into `~/.agents/skills/` before symlinking
- Overwrites `~/.claude/CLAUDE.md` with the `@import`
- Logs where old instructions now live

`init` is idempotent — safe to re-run. It skips any step that's already been done.

> Multi-account support (`~/.claude-<account>/` directories) is planned but not yet implemented.

---

### `lore install <skill> [...]`

Install one or more skills from the current directory.

```
~/.agents/skills/<skill>  →  $PWD/<skill>
```

**On duplicate**, instead of silently overwriting, lore shows both paths:

```
⚠  cooking-chef already installed
   existing  → /old/repo/skills/cooking-chef
   attempted → /new/repo/skills/cooking-chef
```

Errors if the source directory doesn't exist in `$PWD`.

---

### `lore remove <skill> [...]`

Remove skill symlinks from `~/.agents/skills/`. Source directories in repos are never touched.

---

### `lore behavior add <name> [...]`

Install behaviors from the current directory.

For each name, three steps run in order:

1. Creates symlink: `~/.agents/behaviors/<name> → $PWD/<name>`
2. Locates the entry `.md` file inside the folder. Resolution order:
   `RULES.md` → `BEHAVIOR.md` → `README.md` → first `.md` found
3. Appends to `~/.agents/AGENTS.md`:
   ```
   <!-- name -->
   @/absolute/path/to/entry.md
   ```

Each step is independently idempotent.

---

### `lore behavior remove <name> [...]`

Removes the behavior symlink and its `<!-- name --> + @path` block from `AGENTS.md`.

Built-in behaviors (like `from-claude`, created during `init`) are real directories, not symlinks. lore refuses to remove them automatically and instead tells you the exact commands to run manually.

---

### `lore list`

Shows all installed skills and behaviors with their symlink targets.

```
Skills:
  cooking-chef             → /home/you/repos/ai-restaurant/skills/cooking-chef
  broken-skill             → /old/path/that/is/gone  ✗ broken

Behaviors:
  from-claude                (built-in)
  restaurant-rules         → /home/you/repos/ai-restaurant/behaviors/restaurant-rules
```

- Valid symlinks show their target path
- Broken symlinks (pointing to a non-existent path) show `✗ broken`
- Built-in behaviors (real directories, not symlinks) show `(built-in)`

---

### `lore man`

Opens the embedded manual. Uses `$PAGER` (default: `less`), falls back to `cat`.

---

### `lore help`

Prints a short usage summary.

---

## AGENTS.md format

`~/.agents/AGENTS.md` is fully managed by lore. Do not edit it manually.

```markdown
<!-- managed by lore — do not edit -->
<!-- skills auto-loaded from ~/.agents/skills/ -->

<!-- from-claude -->
@/home/you/.agents/behaviors/from-claude/RULES.md

<!-- restaurant-rules -->
@/home/you/.agents/behaviors/restaurant-rules/RULES.md
```

Each behavior is a two-line block:

```
<!-- name -->
@/absolute/path/to/entry.md
```

The `<!-- name -->` comment is what lore uses to locate and remove the block. The `@path` line is what Claude imports. Both lines are always written together and removed together.

---

## Behavior entry file

When `lore behavior add` looks for the entry point inside a behavior folder, it checks in this order:

1. `RULES.md`
2. `BEHAVIOR.md`
3. `README.md`
4. First `.md` file found (alphabetical)

If none are found, lore errors.

---

## Environment

| Variable | Default | Description |
|---|---|---|
| `AGENTS_DIR` | `~/.agents` | Base directory. All derived paths move with it. |
| `PAGER` | `less` | Pager used by `lore man`. |

---

## Files

| Path | Description |
|---|---|
| `~/.agents/AGENTS.md` | Universal agent instructions |
| `~/.agents/skills/` | Skill symlinks |
| `~/.agents/behaviors/` | Behavior symlinks and built-ins |
| `~/.claude/CLAUDE.md` | Written by `init`: single `@import → AGENTS.md` |
| `~/.claude/skills` | Symlink created by `init` |

---

## Testing without touching real config

Use `AGENTS_DIR` to point lore at a throwaway directory:

```bash
AGENTS_DIR=/tmp/lore-test lore init

mkdir -p /tmp/fake-repo/skills/my-skill
touch /tmp/fake-repo/skills/my-skill/SKILL.md
cd /tmp/fake-repo/skills

AGENTS_DIR=/tmp/lore-test lore install my-skill
AGENTS_DIR=/tmp/lore-test lore list
```
