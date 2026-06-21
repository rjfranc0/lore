# Accounts

## What this covers

The config file lore reads its own settings from
(`~/.config/lore/lore.toml`), and the full lifecycle of a "Claude account" —
a wired `~/.claude*/` directory lore knows about and can list, repair, or
forget.

## Domain language

- **Config file**: `~/.config/lore/lore.toml`. Holds exactly two things:
  `agents_dir` (where the universal `~/.agents/` tree lives) and `[accounts]`
  (a name → path registry of every Claude account lore has wired). This file
  is neutral ground — it sits above `~/.agents/` itself, leaving room for
  more than one agents-dir in the future (explicitly a non-goal for now, see
  below).
- **Account**: a name (`default`, or any alphanumeric+hyphen string) mapped
  to a Claude config directory path. The `default` account is special only
  in that `lore init` (no flag) always resolves to it — it is not a
  different *kind* of account, just the one with no name argument.
- **Registered** vs. **wired**: an account is *registered* if it has an
  entry in `lore.toml`'s `[accounts]` table. It is *wired* if `CLAUDE.md`,
  `LORE.md`, and the skills symlink actually exist correctly on disk at
  that path. These can drift apart (disk state changes without the
  registry knowing) — `accounts sync` is what reconciles them back
  together.
- **`LORE.md`**: a file fully owned by lore, one per Claude account
  (`~/.claude/LORE.md` for `default`, `~/.claude-<name>/LORE.md` for a
  named account). It imports the shared `AGENTS.md` and is the thing
  `CLAUDE.md` itself imports — see "CLAUDE.md: surgical, never fully
  overwritten," below.

## Feature: the config file's bootstrap behavior

**What it does**: the very first time anything reads the config (no file
exists yet at `~/.config/lore/lore.toml` or wherever `LORE_CONF` points),
lore does not error — it silently falls back to hardcoded defaults
(`agents_dir = ~/.agents`, no accounts registered yet) and proceeds. The
file only gets created the first time something is registered into it (see
`init`, below).

**Why**: a missing config file must mean "nothing has been set up yet," not
"broken state" — this is what makes `lore init` work identically on a
machine that has never run lore before and one that has a config file
already.

**Acceptance conditions**:
- `LORE_CONF` env var, if set, is the *only* override for the config file's
  path — there is no other lore-specific env var (this replaced an older
  `AGENTS_DIR`/`CLAUDE_DIR` pair, see Decisions below).
- Given no config file exists at all, when any command reads config, then it
  behaves exactly as if a file containing only the hardcoded defaults
  existed — no error, no warning.

## Feature: `lore init` (default account)

**What it does**: bootstraps `~/.agents/` (creates `AGENTS.md`, `skills/`,
`behaviors/` if missing) — independently — and wires the `default` Claude
account: creates/updates `~/.claude/LORE.md` to import `AGENTS.md`,
surgically wires `~/.claude/CLAUDE.md` to import `LORE.md` without ever
overwriting it, and symlinks `~/.claude/skills → ~/.agents/skills`.
Registers `default` in the config the first time it runs.

**Why idempotent**: re-running must be safe — it's the documented recovery
path if `AGENTS.md` ever gets deleted by accident (it gets rebuilt,
re-registering every behavior still present on disk), and `LORE.md`/
`CLAUDE.md` wiring is safe to repeat on every run regardless.

### AGENTS.md: created once, recovered if deleted

If `AGENTS.md` is missing, it's created fresh; if `~/.agents/behaviors/`
already has directories on disk (the recovery scenario), each one is
re-registered into the new `AGENTS.md` and logged — nothing already on
disk is lost just because the index file was. If `AGENTS.md` already
exists, this step is skipped entirely and prints "AGENTS.md exists —
skipping." **This no longer has anything to do with CLAUDE.md's state** —
before `LORE.md` existed, AGENTS.md's creation and CLAUDE.md's migration
were one gated decision; now they're two independent steps that both run
on every `init`.

### LORE.md: always fresh, never stale

`LORE.md` is rewritten on every `init` run, unconditionally — it's fully
lore's own file (nothing else ever writes to it), so there's nothing to
lose by always re-deriving its import line.

### CLAUDE.md: surgical, never fully overwritten

This is the part that runs on *every* `init`, for *every* account,
independent of `AGENTS.md`'s state:

| State of CLAUDE.md | Action |
|---|---|
| Already imports `LORE.md` | Skip — idempotent |
| Imports `AGENTS.md` directly (the legacy, pre-`LORE.md` line) | That one line is replaced with the `LORE.md` import |
| Has other content, no lore import yet | Migrated (see below); the import is appended below the original content |
| Empty or absent | The `LORE.md` import is written as the whole file |

**Migration** (the third row): existing content is copied verbatim to
`~/.agents/behaviors/from-claude/RULES.md`, registered as a behavior named
`from-claude`, and the `LORE.md` import is appended *underneath* the
original content — the original text stays exactly where the user left
it, never replaced. Any line in that original content which itself starts
with `@` (another tool's own import line) is named explicitly in the
migration warning and left untouched, since it isn't lore's to manage.

> Nothing is ever deleted or overwritten in this path — old content is
> always relocated *and* kept live in place, never discarded, and lore
> prints exactly where each piece landed plus every foreign import line it
> saw and left alone.

Any real (non-symlinked) skill directories sitting in `~/.claude/skills/`
are moved into `~/.agents/skills/` before the symlink replaces that path —
unaffected by the `LORE.md` change.

**Skill collision is a hard stop**: if a real skill being migrated from
`~/.claude/skills/` shares a name with one already in `~/.agents/skills/`,
lore leaves both alone, warns about every conflicting name, and refuses to
touch `CLAUDE.md` at all that run — a half-migrated config (some skills
moved, `CLAUDE.md` already pointing at the new setup, others stuck) is worse
than stopping early. The fix is manual: resolve the name conflict, re-run
`init`.

**Acceptance conditions**:
- Given `~/.claude/CLAUDE.md` already imports `LORE.md` (a re-run), when
  `init` runs, then nothing changes — migration only ever happens once per
  account.
- Given `AGENTS.md` already exists, when `init` runs again, then the
  AGENTS.md-creation step is skipped (prints "AGENTS.md exists —
  skipping") — but `LORE.md`/`CLAUDE.md` wiring still runs every time, for
  every account, since (unlike before) it isn't gated on AGENTS.md's
  absence.

## Feature: `lore init --account <name>`

**What it does**: the same bootstrap, but wires `~/.claude-<name>/` instead
of `~/.claude/`, and registers `<name>` (not `default`) in the config.
`~/.agents/` itself is shared — there is exactly one universal
skills/behaviors tree, fanned out to as many Claude accounts as the user
wires.

**Account name rule**: alphanumeric characters and hyphens only. Any other
character (including empty string) is rejected with a clear error **before
any disk operation** — no partial directory gets created for a name that's
about to be rejected.

**`--account default` is a complete alias for omitting the flag** — both
resolve through the exact same registry entry and the exact same
`~/.claude` path. This is a deliberate unification (see Decisions): treating
`default` as just another name would silently wire a second, untracked
`~/.claude-default/` directory invisible to every other `accounts` command,
the moment someone typed the implicit default's name back explicitly.

**Per-account `LORE.md`, shared `AGENTS.md`**: each account gets its own
`LORE.md` (`~/.claude-<name>/LORE.md`), and every account's `LORE.md`
imports the exact same shared `~/.agents/AGENTS.md` — there's still only
one universal skills/behaviors tree. But if *this* account's `CLAUDE.md`
has content to migrate, that content registers into *this account's own*
`LORE.md`, never into the shared `AGENTS.md` — a named account's stray
instructions never leak into every other account's config.

**Acceptance conditions**:
- Given `--account work` runs twice, when the config is inspected, then
  exactly one `work` entry exists (idempotent registration, not a
  duplicate).
- Given two different account names are initialized, when each is
  inspected, then both are fully wired and isolated from each other — same
  `AGENTS.md`/skills tree underneath, independent `LORE.md` + `CLAUDE.md` +
  skills symlink per account.
- Given `~/.claude-work/CLAUDE.md` has content to migrate, when `init
  --account work` runs, then the migrated copy registers in
  `~/.claude-work/LORE.md`, and the shared `~/.agents/AGENTS.md` is left
  untouched.

## Feature: `accounts list`

Prints the registry exactly as stored — name and path, sorted by name, or
`(none)` if empty. This is a pure registry read: it does **not** check
whether anything is actually correct on disk — that's `sync`'s job, kept
as a separate command by spec.

> ⚠️ **Inferred:** the spec confirms the two responsibilities are split
> (`list` has "no disk status check — that belongs to `accounts sync`");
> the specific reason — so `list` stays fast and side-effect-free — is
> this document's own gloss on why, not stated directly anywhere.

## Feature: `accounts remove <name>`

**What it does**: deletes `<name>`'s entry from the config registry only.
Nothing on disk — not the `~/.claude-<name>/` directory, not `CLAUDE.md`,
not the skills symlink — is touched.

**Why**: consistent with lore's broader non-destructive philosophy (see
[@/functional/agent-config.md] — symlinks are never force-deleted either).
Forgetting an account is reversible by hand (the directory is still there);
forgetting it *and* wiping its directory would not be.

**Acceptance conditions**:
- Removing `default` is explicitly allowed, with a warning that re-running
  `lore init` is what re-registers it — `default` is not a protected name
  once it's just "another entry in the registry."
- Removing a name that was never registered warns (not an error) and exits
  0 — "already not there" is the same successful end-state as "removed."

## Feature: accounts sync

**What it does**: for every account in the registry, checks whether it's
actually wired correctly — `CLAUDE.md` imports `LORE.md`, **and** `LORE.md`
itself imports `AGENTS.md`, **and** the skills symlink exists and resolves
to a live directory. Any account that fails any of these checks gets fully
re-wired via the same path `init` uses (the same surgical CLAUDE.md
handling, not a shortcut), and the rewire is reported by name. If every
account was already correct, reports "Accounts already in sync" instead.

**Why**: accounts can break independently of lore (a user deletes a
`CLAUDE.md` or `LORE.md` by hand, a skills symlink target moves) — `sync`
is the repair tool, parallel to the original `lore sync` but scoped to
Claude wiring instead of `AGENTS.md` content. The two `sync` commands are
intentionally separate (`lore sync` vs. `lore accounts sync`) rather than
one command with a flag, to keep each one's blast radius obvious from its
own name.

**Acceptance conditions**:
- Given an account's `CLAUDE.md` is deleted, when `accounts sync` runs, then
  it is recreated with correct content and the rewire is reported.
- Given an account's `LORE.md` is deleted, when `accounts sync` runs, then
  it is recreated with the correct `AGENTS.md` import.
- Given an account's `CLAUDE.md` lost its `LORE.md` import line (but the
  file itself still exists), when `accounts sync` runs, then it's rewired
  back to importing `LORE.md`.
- Given an account's skills symlink is broken (deleted or dangling), when
  `accounts sync` runs, then it is recreated.
- Given every account is already correctly wired (both hops, plus skills),
  when `accounts sync` runs, then nothing is rewritten and it reports as
  such.

> ⚠️ **Inferred:** a read failure on `CLAUDE.md` or `LORE.md` (permission
> denied, non-UTF8 content) is treated the same as "not wired" and triggers
> a rewire, rather than being surfaced as a distinct error. This is a
> deliberate choice (a self-healing command shouldn't need a separate error
> branch for an unreadable-but-fixable file), confirmed during this
> feature's own review pass rather than guessed from code alone.

## Decisions

| Decision | Alternatives considered | Rationale |
|---|---|---|
| Config at `~/.config/lore/` | `~/.agents/lore.toml` | Neutral ground — `~/.agents/` is agent *data*, not lore's own config; leaves room for a future multi-agents-dir setup |
| TOML format | JSON, custom | Native fit for the Rust ecosystem, serde-friendly |
| `accounts sync` separate from `lore sync` | `lore sync --accounts` flag | All account operations live under one noun; `sync` (no namespace) stays AGENTS.md-only |
| Registry-only on `accounts remove` | Also wipe the account's directory from disk | Consistent with the non-destructive philosophy applied everywhere else in this tool |
| Clean break removing `AGENTS_DIR`/`CLAUDE_DIR` env vars | Deprecation warnings first | lore is pre-1.0; no installed base to protect from a breaking change |
| `--account default` unified with the implicit default | Reject `--account default` outright as an error | A silent second untracked directory was strictly worse than either valid option; unification was chosen as the more forgiving of the two |
| `LORE.md` as an intermediary, `CLAUDE.md` import surgical | `CLAUDE.md` keeps directly `@import`-ing `AGENTS.md`, fully overwritten on every `init` | The direct-overwrite model broke coexistence with other tools (e.g. `rtk`) that write their own lines into `CLAUDE.md`. `LORE.md` is the file lore fully owns and can safely overwrite; `CLAUDE.md` becomes shared ground lore only ever appends one line to |

## Non-goals (this domain)

- More than one `agents_dir` — every account shares exactly one universal
  skills/behaviors tree.
- Per-account skill or behavior scoping.
- Disk cleanup on `accounts remove` — by design, see above.
