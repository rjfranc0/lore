# Module: `config.rs`

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
  `None` for an unregistered name; callers (see [init](init.md) below)
  decide what `None` means in context — `config.rs` itself has no opinion
  on fallback behavior.
- `require_account_path(name)` — thin wrapper around `account_path` that
  turns `None` into a contextual `Err` naming the exact fix (`lore init
  --account <name>`) instead of leaving each caller to invent its own
  message. Used by every `--account`-scoped command (`install`, `remove`)
  so the error text is identical regardless of which command hit the
  unregistered name.

**`validate_account_name(name)`** (free function, not a `LoreConfig`
method — it runs before any config is loaded): non-empty,
alphanumeric-or-hyphen only. Extracted so `init`, `install --account`, and
`remove --account` share one rejection rule instead of three copies
drifting independently — see [init](init.md) for the original call site
this was lifted from.

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

## What breaks if this is touched

- `LoreConfig::save`'s parent-dir creation means removing it from
  `config.rs` would break `init`'s account-registration step today, since
  that call site relies on it rather than creating the directory itself.
