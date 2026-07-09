# Feature: behavior add / remove

See [agent-config](index.md) for domain language (**behavior**, **built-in
behavior**, **shared** vs. **scoped**) this feature builds on.

**What it does**: `lore behavior add <name> [...]` does three things per
name, in order: (1) symlinks `~/.agents/behaviors/<name> → $PWD/<name>`, (2)
locates that behavior's entry file, (3) appends a two-line block referencing
it to `AGENTS.md`. `lore behavior remove <name> [...]` reverses both —
deletes the symlink and strips the matching block from `AGENTS.md`. This is
the **shared** path (no `--account`), unchanged from before per-account
scoping existed.

**Scoped** (`--account <name>`): the same three steps, but entirely inside
one Claude account instead of the shared tree — `~/.claude-<name>/behaviors/<name>
→ $PWD/<name>` and a block appended to that account's own `LORE.md`
(`~/.claude-<name>/LORE.md`), via `claude_behaviors_path` (see
[@/implementation/accounts/wire.md#module-wirers]). `~/.agents/AGENTS.md` and
every other account are left completely untouched — mirrors the
shared-by-default/opt-in-scoped shape of
[skill install/remove](skills.md#feature-skill-install--remove),
except a behavior has no shared+fan-out mode: it's shared *or* scoped to
exactly one account, never both in one command.

**`--account` targets must already be wired**: `lore behavior add --account
work <name>` requires `~/.claude-work/LORE.md` to already exist (i.e. `lore
init --account work` must have run) — it fails with a clear message naming
`lore init --account work` and creates nothing (no symlink, no partial
write) if that account isn't wired yet.

**Why**: a skill is meaningless to an agent until something tells the agent
to load it — behaviors are that "something" for always-on instructions
specifically (as opposed to skills, which agents discover and load
themselves). Scoping mirrors the skill feature's rationale: most behaviors
are universal, but some standing instructions only make sense for one
identity.

**Entry file resolution** (which file inside the behavior directory becomes
the `@import` target): `RULES.md` → `BEHAVIOR.md` → `README.md` → first
`.md` file alphabetically. If none exist, `add` fails outright — a behavior
with no resolvable entry point cannot be wired into `AGENTS.md`/`LORE.md` at
all. Identical resolution order for both the shared and scoped paths.

**Acceptance conditions**:
- Requires `lore init` to have already run (`AGENTS.md` must exist) —
  shared `behavior add`/`remove` fail immediately with "Run 'lore init'
  first" otherwise. This is a precondition, not a recoverable warning.
  The scoped path has its own equivalent precondition: that account's
  `LORE.md` must exist (see above).
- Given a behavior is already symlinked, when `add` runs again (shared or
  scoped), then the symlink step is skipped (idempotent) but the
  AGENTS.md/LORE.md-membership check still runs independently — each of the
  two steps (symlink, registry entry) is idempotent on its own, not just the
  command as a whole.
  > ⚠️ **Inferred:** this independence looks deliberate (it lets a
  > half-completed prior run — symlink created but AGENTS.md write failed —
  > self-heal on retry) but is not stated anywhere explicitly; verify with
  > the original author if this matters for a future change.
- Removal matches by **exact behavior name**, not prefix/pattern — removing
  `a.c` must never also remove `axc`. This is enforced at the parser level
  (shared for both `AGENTS.md` and `LORE.md`, since both are `AgentsMd`
  instances), see [@/implementation/agent-config.md#agentsmd-format].
- A trailing slash on the name (`lore behavior add my-rule/`) is stripped
  before use, for both the shared and `--account` paths — same rule as
  skill install/remove.
- **Built-in behaviors cannot be removed automatically.** `from-claude` is a
  real directory, not a symlink — `behavior remove from-claude` (shared) or
  `behavior remove --account work from-claude` (scoped, if a `from-claude`
  migration happened for that account) detects this and prints the exact
  manual commands instead of acting, rather than guessing whether deleting a
  non-symlinked directory is safe. The scoped message points at that
  account's own `LORE.md`, not the shared `AGENTS.md`.

**Example** (shared, verified against the built binary):
```
⚠  from-claude is a built-in behavior — remove manually:
  rm -rf /home/you/.agents/behaviors/from-claude
  Then remove its <!-- from-claude --> block from /home/you/.agents/AGENTS.md
```
