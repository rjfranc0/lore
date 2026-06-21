# Accounts — Implementation

## Module: `config.rs`

**Responsibility**: `LoreConfig` is the single struct mapped onto
`lore.toml`. Owns reading, parsing, defaulting, and writing that file — no
other module touches the file directly.

**Shape**:
```rust
pub struct LoreConfig {
    pub agents_dir: String,            // String, not PathBuf — needed for clean TOML serde
    #[serde(default)] pub accounts: BTreeMap<String, String>,
}
```
`BTreeMap` (not `HashMap`) is deliberate — accounts print in a stable,
sorted order everywhere they're displayed (`accounts list`, `lore.toml`
itself) without a separate sort step.

**Contracts**:
- `config_path()` — reads `LORE_CONF` if set, else
  `~/.config/lore/lore.toml`. The *only* env var lore reads anywhere in the
  codebase.
- `load_or_default(path)` — **a missing file is not an error.** Returns
  `Self::default()` silently. Only a file that *exists but fails to parse*
  returns `Err`. Every caller relies on this — there is no separate "first
  run" code path anywhere else in the codebase; first-run-ness is entirely
  absorbed by this one fallback.
- `Default` — `agents_dir` = `dirs::home_dir()/.agents`; `accounts` = empty
  map. (Not yet registering `default` — that registration happens in
  `init`, not here; loading the default config does not, by itself, mean
  an account has been wired.)
- `save(path)` — creates every missing parent directory before writing
  (`create_dir_all(path.parent())`), so callers never need their own
  `mkdir` step before saving config. This is why `init`'s
  account-registration step does not call `create_dir_all` itself — doing
  so would be redundant dead code (an explicit decision made and reviewed
  during this feature's build, not an oversight).
- `account_path(name)` — `Option<PathBuf>`, a plain map lookup. Returns
  `None` for an unregistered name; callers (see `init.rs` below) decide
  what `None` means in context — `config.rs` itself has no opinion on
  fallback behavior.

## Module: `paths.rs`

**Shape** (no `claude_dir` field — removed deliberately, see Decisions in
[@/functional/accounts.md#decisions]; only `init.rs` ever needs a
Claude-side path, and it computes that itself):
```rust
pub struct Paths { pub agents_dir, pub skills_dir, pub behaviors_dir, pub agents_md: PathBuf }
```

Two constructors, used in different situations:
- **`Paths::load()`** — reads config from disk/env itself
  (`LoreConfig::config_path()` → `load_or_default()`), then derives paths.
  Used by every command that has *not* already loaded a config (`install`,
  `remove`, `behavior`, `list`, `sync` — none of these need `LoreConfig` for
  anything but path derivation).
- **`Paths::from_config(&config)`** — takes an already-loaded `LoreConfig`.
  Used by `init` and `accounts::*`, which need the loaded config anyway (to
  read/mutate `accounts`) and would otherwise read the same file twice.

All four derived paths are simple joins on `agents_dir` (`skills`,
`behaviors`, `AGENTS.md`) — there is no independent source of truth for
these three paths beyond this one function; nothing else in the codebase
re-derives them.

## Module: `wire.rs`

**Responsibility**: the only place that knows the on-disk layout of a
Claude config directory, and the only place that actually writes one.

```rust
pub fn claude_md_path(claude_dir: &Path) -> PathBuf       // claude_dir.join("CLAUDE.md")
pub fn lore_md_path(claude_dir: &Path) -> PathBuf          // claude_dir.join("LORE.md")
pub fn claude_skills_path(claude_dir: &Path) -> PathBuf    // claude_dir.join("skills")

pub fn wire_lore_md(agents_md: &Path, claude_dir: &Path) -> Result<PathBuf>
pub fn wire_claude_md(claude_dir: &Path, agents_md: &Path,
                       migration_behaviors_dir: &Path, migration_register_md: &Path) -> Result<()>
pub fn wire_claude_skills(skills_dir: &Path, claude_dir: &Path) -> Result<()>
pub fn wire_claude_dir(agents_md: &Path, skills_dir: &Path, claude_dir: &Path,
                        migration_behaviors_dir: &Path, migration_register_md: &Path) -> Result<()>
```

**`LORE.md` is lore's fully-owned intermediary** between the universal
`AGENTS.md` and the shared, partly user-owned `CLAUDE.md`. `wire_lore_md`
reuses the `AgentsMd` struct as-is (its header + named-block format is
byte-identical to what `LORE.md` needs) — loads the existing file if
present, **unconditionally overwrites just the header** to
`@{agents_md}\n`, and saves. The unconditional overwrite is what makes it
idempotent without a separate "is it already correct" branch — nothing
foreign can ever land in `LORE.md`'s header, so there's nothing to
preserve there. `md.behaviors` (any blocks added by a prior
`behavior add --account`-style registration) is left untouched.

**`wire_claude_md` never fully overwrites `CLAUDE.md`.** It normalizes the
path first (a symlink or directory sitting where CLAUDE.md should be is
removed, exactly as the old `wire_claude_dir` did), reads whatever content
remains (`None` if absent), then applies this priority order — order
matters, each case returns before the next is checked:

1. A line already equals (trimmed) `@{lore_md}` → already wired, no-op.
2. A line already equals (trimmed) `@{agents_md}` (the legacy pre-LORE.md
   direct import) → that one line is replaced with `@{lore_md}`, nothing
   else in the file is touched.
3. Content exists and is non-empty after trimming → migrate (see below).
4. Otherwise (absent, or present but empty) → the entire file becomes
   `@{lore_md}\n`.

**Migration** (private `migrate_claude_md`) copies the *original* content
verbatim into `migration_behaviors_dir/from-claude/RULES.md`, registers a
`from-claude` block into `migration_register_md` (guarded by
`contains_name` so a retry never double-registers), then appends
`@{lore_md}` to that same original content and writes the result back —
the old text is never deleted, only added to. It also reports every line
in the original content that (trimmed) starts with `@`: those belong to
other tools that already write their own lines into `CLAUDE.md`, and are
named in the migration warning specifically so the user knows lore saw
them and left them alone.

**`migration_behaviors_dir`/`migration_register_md` are caller-supplied,
not derived here** — `wire_claude_md` doesn't know or care which account
it's wiring. For the default account the caller passes the shared
`~/.agents/behaviors/` and `~/.agents/AGENTS.md`; for a named account it
passes that account's own `<claude_dir>/behaviors/` and that account's own
`LORE.md` — see `commands/init.rs` below for where that split happens.

`wire_claude_skills` is the skills-symlink half, unchanged from before this
file split it out of `wire_claude_dir`: clear whatever currently sits at
`claude_dir/skills` (symlink, directory, or plain file) and create a fresh
symlink to `skills_dir`.

`wire_claude_dir` is the orchestrator, in a fixed order:
`create_dir_all(claude_dir)` → `wire_lore_md` → `wire_claude_md` →
`wire_claude_skills`. The order is load-bearing: `LORE.md` must exist
before `wire_claude_md` runs, because cases 1/2/4 above all write a line
that names it, and a Case-3 migration for a named account also registers
into that same freshly-ensured `LORE.md`.

**`claude_md_path`/`lore_md_path`/`claude_skills_path` exist as the single
source of truth for those joins** — every caller that needs to know where
a Claude account's `CLAUDE.md`, `LORE.md`, or skills symlink lives
(`init.rs`, `accounts::sync`, and `wire.rs` itself) calls through these
three functions rather than independently writing `claude_dir.join(...)`.
This was a deliberate de-duplication: the layout rule used to be computed
in multiple places independently, which is a correctness risk (multiple
places to keep in sync, not just lines to keep short) — see "What breaks
if this is touched," below.

## Module: `commands/init.rs`

**Responsibility**: the only place that resolves *which* Claude directory a
given invocation targets, runs the migration logic, and triggers
registration.

**Validate before any disk I/O**: account name validation (non-empty,
alphanumeric-or-hyphen only) happens before `LoreConfig` is even loaded.
This ordering is itself the contract, not just a nice-to-have — a name
that's about to be rejected must leave zero trace (no directory, no
registry entry, no partial config write).

**`claude_dir` resolution**:
```rust
let account_name = account.clone().unwrap_or_else(|| "default".to_string());
let claude_dir = if account_name == "default" {
    config.account_path("default").unwrap_or_else(|| home_dir().join(".claude"))
} else {
    home_dir().join(format!(".claude-{account_name}"))
};
```
The branch is on **`account_name == "default"`**, not on
`account.is_some()`. This is what makes `--account default` and omitting
the flag resolve identically — both paths land on the exact same
`config.account_path("default")` lookup. Branching on `is_some()` instead
(treating any explicit `--account` the same way, default included) is the
bug this code shape specifically avoids: it would silently wire a second,
unregistered `~/.claude-default/` the moment `default` was passed
explicitly while already registered elsewhere. See
[@/functional/accounts.md#decisions] for why unification was chosen over
rejecting the collision outright.

**AGENTS.md creation is fully decoupled from CLAUDE.md's state.** Before
the `LORE.md` indirection, an internal `should_migrate` check gated
AGENTS.md's Case-1-vs-Case-2 split on whatever was sitting in CLAUDE.md,
which meant migration only ever fired once — for whichever account
happened to run `init` first. That check is gone: `if
!p.agents_md.exists()` now unconditionally creates AGENTS.md fresh
(re-registering any behaviors already on disk, the recovery path) with no
branch on CLAUDE.md at all. CLAUDE.md handling is entirely
`wire_claude_md`'s job now (see `wire.rs` above), invoked via
`wire_claude_dir` on *every* `init` run, for *every* account — not a
one-time thing gated on AGENTS.md's absence.

**Migration target**: just before that `wire_claude_dir` call, `init.rs`
picks which behaviors-dir/register-file pair migrated content should land
in, keyed on `account_name == "default"`: the shared `p.behaviors_dir` /
`p.agents_md` for the default account, or that account's own
`<claude_dir>/behaviors` / `wire::lore_md_path(&claude_dir)` for a named
one. This split is what keeps a named account's migrated instructions from
ever touching the shared `AGENTS.md`.

**Skill migration collision**: while moving real (non-symlinked) skill
directories out of `claude_skills` into `skills_dir`, any name that already
exists at the destination is left in place at the source, warned about, and
flagged via a `collision` bool. After the loop, if `collision` is true
**and** the source directory still has unmoved entries, the whole command
bails — after attempting every movable skill (partial progress is
preserved and reported) but before wiring `CLAUDE.md` (the command never
finishes "successfully" with conflicts still unresolved).

> ⚠️ **Inferred:** the ordering itself is read directly from the code; that
> it's *deliberate* is not — there's no comment explaining why the bail
> happens after the loop rather than on the first collision. The ordering's
> effect (partial progress preserved, `CLAUDE.md` never wired while
> conflicts are outstanding) holds regardless of whether it was a conscious
> design choice or a side effect of the loop's natural structure.

**Registration**: only inserts into `config.accounts` if the key isn't
already present — `init` never overwrites an existing registry entry's
path, even if the resolved `claude_dir` were somehow to differ from what's
stored (it can't, today, given the resolution logic above, but the guard
costs nothing and removes one way a future change could silently relocate
an account's registry entry out from under it).

## Module: `commands/accounts.rs`

**`list`** — pure read of `config.accounts` (already sorted, `BTreeMap`),
no disk check.

**`remove`** — pure mutation of `config.accounts`, then `save`. Warns (does
not error) for an unregistered name or for `"default"` specifically, but
performs the removal either way for `"default"` (the warning is
informational, not a refusal).

**`sync`** — for each registered account, the wiring check now verifies
*two* hops instead of one: CLAUDE.md must import LORE.md, **and** LORE.md
must import AGENTS.md, in addition to the unchanged skills-symlink checks:
```rust
let already_wired = claude_md.exists()
    && std::fs::read_to_string(&claude_md)
        .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", lore_md.display())))
    && lore_md.exists()
    && std::fs::read_to_string(&lore_md)
        .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", agents_md.display())))
    && symlink::is_link(&claude_skills)
    && symlink::is_live(&claude_skills);
```
**A read failure on either file (permission denied, non-UTF-8 content)
folds into `false` via `is_ok_and`** — treated identically to "wrong
content," not surfaced as a distinct error. This is deliberate, for the
same reason as before the two-hop check was added: `sync`'s whole purpose
is self-healing, so routing every form of "not correct" through the same
`wire_claude_dir` rewrite path (rather than carving out a separate branch
for unreadable-but-possibly-fixable files) keeps the function's logic to
one path instead of two. If `wire_claude_dir` itself then fails (e.g.
genuine permission denial on write), that error still propagates normally
— only the *read* used for the "is it already correct" check is
swallowed, not the *write* used to fix it.

If not already wired, `sync` computes the same migration-target tuple
`init.rs` does (keyed on `name == "default"`) before calling
`wire_claude_dir` — a rewire triggered by `sync` goes through the exact
same surgical CLAUDE.md handling `init` does, never a separate code path.

## What breaks if this is touched

- Reverting the `claude_md_path`/`claude_skills_path` centralization (going
  back to inline `.join(...)` calls in `init.rs`/`accounts.rs`)
  reintroduces the duplicated-knowledge risk these helpers were added to
  close — a future layout change would again need to be applied in three
  places by hand.
- Changing `init.rs`'s branch back to `account.is_some()` reintroduces the
  `--account default` collision: a fully-wired, registry-invisible
  `~/.claude-default/` directory.
- `LoreConfig::save`'s parent-dir creation means removing it from
  `config.rs` would break `init`'s account-registration step today, since
  that call site relies on it rather than creating the directory itself.
- Calling `wire_claude_md` before `wire_lore_md` inside `wire_claude_dir`
  breaks every case that writes a `@{lore_md}` line — `LORE.md` wouldn't
  exist yet at the path being named, and a Case-3 migration for a named
  account would have nothing to register into.
