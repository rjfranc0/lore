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
pub fn claude_md_path(claude_dir: &Path) -> PathBuf      // claude_dir.join("CLAUDE.md")
pub fn claude_skills_path(claude_dir: &Path) -> PathBuf   // claude_dir.join("skills")
pub fn wire_claude_dir(agents_md: &Path, skills_dir: &Path, claude_dir: &Path) -> Result<()>
```

`wire_claude_dir`'s steps, in order: `create_dir_all(claude_dir)` → write
`CLAUDE.md` with `@{agents_md}\n` → if a skills symlink already exists,
remove it → create a fresh symlink `claude_dir/skills → skills_dir`. Always
rewrites both, even if only one half was actually broken — idempotent and
cheap enough (local file I/O, on-demand only) that the codebase accepts the
small redundancy rather than detecting which half needs fixing.

**`claude_md_path`/`claude_skills_path` exist as the single source of truth
for that join** — every caller that needs to know where a Claude account's
`CLAUDE.md` or skills symlink lives (`init.rs`, `accounts::sync`, and
`wire_claude_dir` itself) calls through these two functions rather than
independently writing `claude_dir.join("CLAUDE.md")`. This was a deliberate
de-duplication: the layout rule used to be computed in three places
independently, which is a correctness risk (three places to keep in sync,
not just three lines to keep short) — see "What breaks if this is
touched," below.

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

**Migration** (`should_migrate`): true only when `claude_md` exists, has
non-empty content, **and** that content does not already contain lore's own
`@{agents_md}` import line. This three-part check is what prevents a
second `init` run from re-triggering Case 2 against lore's own
previously-written `CLAUDE.md`.

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

**`sync`** — for each registered account, the wiring check is:
```rust
let already_wired = claude_md.exists()
    && std::fs::read_to_string(&claude_md).is_ok_and(|c| c.contains(&format!("@{}", agents_md.display())))
    && symlink::is_link(&claude_skills)
    && symlink::is_live(&claude_skills);
```
**A `CLAUDE.md` read failure (permission denied, non-UTF-8 content) folds
into `false` via `is_ok_and`** — treated identically to "wrong content,"
not surfaced as a distinct error. This is deliberate: `sync`'s whole
purpose is self-healing, so routing every form of "not correct" through the
same `wire_claude_dir` rewrite path (rather than carving out a separate
branch for unreadable-but-possibly-fixable files) keeps the function's
logic to one path instead of two. If `wire_claude_dir` itself then fails
(e.g. genuine permission denial on write), that error still propagates
normally — only the *read* used for the "is it already correct" check is
swallowed, not the *write* used to fix it.

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
