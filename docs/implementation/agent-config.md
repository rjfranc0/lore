# Agent Config — Implementation

## Module: `agents_md.rs`

**Responsibility**: parse, mutate, and re-serialize `AGENTS.md`'s exact
on-disk format. This is the only module that understands that format —
every command that touches `AGENTS.md` goes through `AgentsMd`, never raw
string manipulation.

**Shape**:
```rust
pub struct AgentsMd { pub header: String, pub behaviors: Vec<BehaviorBlock> }
pub struct BehaviorBlock { pub name: String, pub path: PathBuf }
```

### AGENTS.md format

Exactly two lines per behavior, no exceptions:
```
<!-- name -->
@/absolute/path/to/entry.md
```
Everything before the first such comment block is the **header** (preserved
verbatim on round-trip, including the "managed by lore — do not edit"
banner). A block comment is recognized only as an *exact* `<!-- name -->`
line — `name` must be non-empty and contain no spaces
(`parse_block_comment`); anything else is treated as ordinary header/body
text, not a behavior block.

**Example** (a fully-wired file with two behaviors):
```markdown
<!-- managed by lore — do not edit -->
<!-- skills auto-loaded from ~/.agents/skills/ -->

<!-- from-claude -->
@/home/you/.agents/behaviors/from-claude/RULES.md

<!-- restaurant-rules -->
@/home/you/.agents/behaviors/restaurant-rules/RULES.md
```

**Round-trip invariant**: `parse(content).serialize() == content` for any
well-formed input — verified directly by the `round_trip` unit test.
Anything that breaks this (e.g. changing whitespace handling) is a
regression, not a style choice.

**`remove_by_name` matches by exact string equality**, not prefix or
substring — this is what guarantees removing behavior `a.c` can never also
remove `axc`. Implemented as `Vec::retain(|b| b.name != name)`; there is no
name-normalization (case folding, trimming) anywhere in this path, so a
name must match exactly as it was registered.

**`behavior_entry(dir)` resolution order**: `RULES.md` → `BEHAVIOR.md` →
`README.md` → first `.md` file in the directory, alphabetically. Returns an
error if none exist — a behavior directory with zero markdown files cannot
be wired into `AGENTS.md`, by design (there would be nothing for `@import`
to point at).

## Module: `symlink.rs`

**Responsibility**: the only symlink primitives the rest of the codebase
needs. Deliberately tiny — four free functions, no struct.

```rust
pub fn create(src: &Path, dst: &Path) -> Result<()>   // unix-only; bails on other platforms
pub fn is_link(path: &Path) -> bool                     // true even for a broken symlink
pub fn is_live(path: &Path) -> bool                      // true only if the link's target exists AND is a directory
pub fn is_live_file(path: &Path) -> bool                 // true only if the link's target exists AND is a regular file
```

**The `is_link` / `is_live` split is the key invariant every caller relies
on.** `is_link` uses `symlink_metadata` (does *not* follow the link) so it
answers "is there a symlink here at all," true regardless of whether the
target exists. `is_live` uses `Path::is_dir` (*does* follow the link) so it
answers "does the thing this points at actually exist as a directory right
now." A symlink is **broken** exactly when `is_link(path) && !is_live(path)`
— this exact combination is how `lore list` decides whether to print
`✗ broken`, how `accounts sync` (see
[@/implementation/accounts/index.md]) decides whether an account needs
re-wiring, and how `lore update --all` finds its candidates (see
[@/functional/agent-config/update.md#feature-update]) — three independent inline
copies of the same check, not a shared helper. Collapsing these two checks
into one (e.g. just using `is_dir()`
everywhere) would make a broken symlink indistinguishable from "nothing is
here," which is a different state `list` needs to report differently.

**`is_live_file` is the file-shaped twin of `is_live`**, added for subagents
(a single `.md` file, not a directory). It is `Path::is_file` — also
follows the link — and `is_live` is deliberately left directory-only so
skills and behaviors keep their semantics. The broken predicate for an agent
is therefore `is_link(path) && !is_live_file(path)`; a link that now points
at a *directory* counts as broken for agents, exactly as a link to a file
counts as broken for skills. `list` and `update --all` each pick the
predicate by entry kind rather than duplicating the loop.

## Module: `output.rs`

Three formatting helpers used everywhere a command needs to print a result
line: `ok` (`✓`), `warn` (`⚠ `), `note` (two-space indent, for sub-details
under a `warn`/`ok` line — e.g. showing both the existing and attempted
symlink target on an install collision). No logic, purely consistent
prefixing.

> ⚠️ **Inferred:** the module carries no comments stating its purpose;
> "exists so every command's output looks the same without each one
> hand-formatting its own checkmarks" is this document's own read of why
> three one-line functions exist instead of inline `println!`s at each
> call site.

## Commands built on these primitives

`commands/install.rs`, `remove.rs`, `behavior.rs`, `list.rs`, `sync.rs`,
`update.rs` all start with `Paths::load()` (see
[@/implementation/accounts/config.md#module-pathsrs] — `Paths` is owned by the
config layer, not this one, since it has to know about `LoreConfig` to
resolve `agents_dir`) — `update.rs` differs only in shape, not substance:
its core logic (`update_one`/`update_all`) takes `&Paths` and the resolved
`cwd` as plain parameters instead of calling `Paths::load()`/
`std::env::current_dir()` internally, so it stays unit-testable against a
`tempfile::tempdir()`-backed `Paths` with no real cwd or env dependency;
only the thin `run()` wrapper touches that I/O. From there:

- **install/remove** (skills): each takes an `account: Option<String>` and
  branches on it. **Shared** (`None`): the original thin
  `symlink::create`/`is_link` logic against `~/.agents/skills/`, plus a
  fan-out loop over every registered account calling
  `wire::relink_skill`/`unlink_account_skill` (see
  [@/implementation/accounts/wire.md#module-wirers]) so the same install/remove
  reaches every account in one command. **Scoped** (`Some(name)`): resolves
  `name` via `LoreConfig::require_account_path` (bailing if unregistered)
  and applies the same symlink-create/remove logic directly against that
  one account's skills dir instead of the shared one — `~/.agents/skills/`
  and every other account are never touched. The **scoped** path validates
  `name` via `config::validate_account_name` before resolving it, same as
  every other `--account`-scoped command. Both paths (and `behavior
  add`/`remove`'s scoped/shared functions below) share one trailing-slash
  strip, `commands::normalize_name` (`raw.trim_end_matches('/')`, defined
  once in `commands/mod.rs`) — factored out of what used to be independent
  inline copies per command — so tab-completion's `my-skill/` and a
  hand-typed `my-skill` resolve to the same path everywhere a name is taken
  as an argument.
- **agent add/remove** (`commands/agent.rs`): the skill install/remove shape
  applied to `<name>.md` files. Unlike the commands above it loads a
  `LoreConfig` first (it fans out over `config.accounts`) and derives paths
  via `Paths::from_config`. A private `normalize_agent_name(raw)` applies
  `commands::normalize_name` (trailing `/`) and then strips **one** trailing
  `.md` (`strip_suffix(".md").unwrap_or(name)`), so `reviewer.md.md` names the
  agent `reviewer.md`; every path is then `<name>.md`, and output prints the
  bare name.

  `add(names, account)` resolves one target dir: `Paths::subagents_dir`
  (shared), or — scoped — the private `scoped_agents_dir(config, account)`:
  `validate_account_name` → `require_account_path` →
  **`wire::agents_dir_is_symlink` check** → `wire::claude_agents_path`, all
  before `create_dir_all(target)`, so an unregistered account or a
  user-symlinked `agents/` fails with nothing created. The symlink case
  `bail!`s (exit 1, through the existing `anyhow` error printer) with
  `<path> is a symlink — lore does not modify through a user-managed agents
  directory; replace it with a real directory to use --account`: a symlinked
  `agents/` may alias the shared pool or another account, so writing through
  it would break `--account`'s isolation (see
  [@/implementation/accounts/wire.md#module-wirers]). Per name, `src = cwd/<name>.md`, `dst = target/<name>.md`, in this
  order: `src` not a file → warn `'<name>.md' not found in <cwd>` and
  `continue` (no fan-out for that name); `dst` is a symlink → warn `<name>
  already installed` with `existing`/`attempted` notes (same format as
  `install`), never overwritten; `dst` exists but isn't a symlink → warn it is
  not a symlink and skip; otherwise `symlink::create` and `Installed agent
  <name>`. **Fan-out is shared-only and runs after any of the last three
  outcomes**, not just after a fresh install: `wire::relink_agent` over every
  `config.accounts` entry (default included, since it is registered), which
  self-heals a missing re-link and is a no-op where one exists. Scoped `add`
  never fans out. For an account whose `agents/` is a symlink,
  `relink_agent` warns and skips (returns `Ok`), so the fan-out carries on
  over the remaining accounts — a per-account skip, where scoped commands
  fail outright.

  `remove(names, account)` mirrors it. Shared: remove `subagents_dir/<name>.md`
  if `is_link`, else warn `<name> is not installed`; then
  `wire::unlink_account_agent` over every account **regardless** of whether the
  shared link existed, so orphaned re-links are still cleaned (an account with
  a symlinked `agents/` is warned about and skipped; when that symlink aliases
  the shared pool, the shared link's removal already removed its content).
  Scoped: resolve the account through the same `scoped_agents_dir` (so the
  symlink refusal applies before anything is removed), remove the link if
  `is_link`, else warn
  `<name> is not installed in account '<account>'` — a real file there is
  therefore reported as "not installed" and left in place. Source files are
  never touched.
- **behavior add/remove**: like install/remove, each takes an `account:
  Option<String>` and dispatches — `add`/`remove` are thin wrappers that
  branch to `add_shared`/`add_scoped` or `remove_shared`/`remove_scoped`.
  Those four are themselves thin: each resolves its own `behaviors_dir`
  and markdown-file path (`~/.agents/behaviors/` + `AGENTS.md` for shared;
  `wire::claude_behaviors_path(&claude_dir)` + that account's `LORE.md`
  for scoped, after resolving `name` via `LoreConfig::require_account_path`
  and guarding on the account's `LORE.md` already existing — bailing with
  a `lore init --account <name>` pointer if not) and hand off to one
  shared implementation: `link_and_register` (symlink-create/register,
  used by both add paths) and `unlink_and_deregister` (symlink-remove/
  deregister, used by both remove paths), parameterized over that
  directory/markdown-file pair plus a label or not-installed-message
  closure for the shared-vs-scoped wording difference — so the actual
  link+register / unlink+deregister business rule lives in exactly one
  place regardless of which tree it targets. The shared tree is never
  touched by a scoped call, and vice versa. `unlink_and_deregister`
  additionally distinguishes a symlinked behavior (removable) from a real
  directory (`is_link` false but `path.is_dir()` true) — that's the
  built-in-behavior protection described in
  [@/functional/agent-config/behaviors.md#feature-behavior-add--remove]; the scoped
  call's warning names that account's `LORE.md` path instead of the
  shared `AGENTS.md`.
- **sync**: two passes sharing one helper, `reconcile_behaviors(md: &mut
  AgentsMd, behaviors_dir: &Path) -> Result<(Vec<String>, Vec<String>)>`.
  The helper mutates `md` in place — drops any entry whose
  `behaviors_dir.join(&b.name)` isn't a directory (stale, via
  `md.remove_by_name`), then walks `behaviors_dir` on disk and adds any
  directory not yet in `AgentsMd` (missing, via `md.add`/
  `md.contains_path`) — a single pass each direction, no cross-checking
  beyond directory existence — and returns `(added, removed)` names without
  printing or saving, so each caller owns its own messaging/persistence.
  Pass 1 calls it against the shared `AgentsMd`/`~/.agents/behaviors/`
  exactly as before (byte-identical messages: `Removed stale entry: {name}`
  / `Added {name} to AGENTS.md`). Pass 2 loops `config.accounts`
  (`BTreeMap`, alphabetical), and for each one whose `LORE.md` exists
  (missing `LORE.md` is a `warn`-and-`continue`, not a bail): compares
  `md.header`'s first line against the canonical `@{agents_md}\n` and
  restores it on drift, then calls the same helper against
  `wire::claude_behaviors_path(&claude_dir)` with per-account wording
  (`Removed stale entry from {name}: {n}` / `Added {name}/{n} to
  LORE.md`). A `total_changes` counter spans both passes: zero total
  changes always prints one collapsed `✓ Already in sync`, regardless of
  how many accounts are registered; otherwise it emits granular per-target
  "already in sync" lines for whichever targets didn't change.
- **list**: a shared `collect_dir_entries(dir, indent, real_dir_label,
  skip_relink_target)` helper reads a single directory, sorted by filename,
  rendering target + liveness for symlinks or a `(migrated)`/`(built-in)`
  tag for real directories, `(none)` if nothing qualified, and returns
  `(rendered_text, found_any)` so callers can also decide whether a section
  is worth printing at all (`print_dir_entries` is a thin wrapper that
  discards the bool for the shared, always-printed sections). Both helpers
  also take a private `EntryKind { Dir, MdFile }`: existing skill/behavior
  call sites pass `Dir` and behave exactly as before; `MdFile` (agents)
  changes three things — liveness uses `symlink::is_live_file`, the displayed
  name drops a trailing `.md`, and **non-symlink entries are skipped
  entirely** (no `(migrated)`/`(built-in)` tag, `found` stays false), so a
  hand-written `.md` in an account's `agents/` is never listed. The
  `skip_relink_target` comparison still uses the raw entry name, so it keeps
  matching `subagents_dir/<name>.md`. `run()` calls
  it once each for the shared `skills_dir`/`behaviors_dir`/`subagents_dir`
  (`Shared skills:`/`Shared behaviors:`/`Shared agents:`, no filtering), then
  once each per
  registered account — **`default` included** — via `wire::claude_skills_path`/
  `claude_behaviors_path`/`claude_agents_path`, i.e. the exact same
  account-directory resolution
  every other account gets, not a hardcoded shared-tree path. The account
  skills call passes `skip_relink_target: Some(&p.skills_dir)` — entries
  whose symlink target is exactly `skills_dir.join(name)` are shared-skill
  re-links (see `wire::relink_skill`, [@/implementation/accounts/wire.md#module-wirers]),
  not account-owned installs, so they're skipped there since they're
  already printed once under `Shared skills:`. The account agents call does
  the same with `Some(&p.subagents_dir)`, for the same reason
  (`wire::relink_agent` re-links). The account behaviors call
  passes `None`: `behavior add --account` (see above) always symlinks
  straight to the source repo, never through a shared re-link, so no such
  entry can exist to filter.

  **Only the `Account: <name>` header's print is conditional, not the loop
  itself**: a section is skipped exactly when `name == "default"` *and*
  all of `has_skills`/`has_behaviors`/`has_agents` (the bool each
  `collect_dir_entries` call returns) come back false. Any other account
  always gets a section,
  even when fully empty (rendered as three `(none)` sub-sections) — that's
  what distinguishes "registered but empty" from "not registered" for a
  named account. For `default` specifically, staying silent only when it
  has nothing account-specific to show is what keeps a bare `lore init`
  from growing a permanent empty `Account: default` section, while still
  surfacing a skill or behavior installed with `--account default` under
  its own section, exactly like any other account.
- **update**: `locate` checks `skills_dir`, then `behaviors_dir`, then
  `subagents_dir` for a given name; the agent lookup strips one trailing
  `.md` from the name first (`agent_stem`) and checks
  `subagents_dir/<stem>.md` with `is_link || is_file`, returning
  `Kind::Agent`. A `Kind` enum (`Skill`/`Behavior`/`Agent`) carries the type
  through; its `is_valid_source` (file for `Agent`, directory otherwise) and
  `source_noun` are shared by `update_one` (an agent's default source is
  `cwd/<stem>.md`; a missing or wrong-shaped source bails `'<path>' not
  found`) and `relink_candidate` (a wrong-shaped answer warns `'<path>' is
  not a file`/`a directory`, skips, and continues). Relinking is
  unconditional — it never checks current link
  health first, just removes any existing symlink and recreates it (the
  same force semantics apply whether the old link was broken or healthy).
  Agents never touch `AGENTS.md`: `sync_behavior_entry` is only reached for
  `Kind::Behavior`.
  For a behavior, `sync_behavior_entry` re-runs `behavior_entry` against
  the new target and rewrites the `AGENTS.md` block only if the resolved
  filename actually changed. `--all` finds broken candidates with the same
  `is_link && !is_live` predicate `list` uses to flag `✗ broken` —
  `find_broken(dir, is_live)` takes the liveness check as a plain `fn(&Path)
  -> bool` parameter: skills and behaviors pass `symlink::is_live`, and a
  third pass over `subagents_dir` passes `symlink::is_live_file` (entries
  displayed with `.md` stripped). Unlike `list` it does not sort them —
  prompt order follows
  `std::fs::read_dir`'s unspecified order within each directory (skills
  are always prompted before behaviors, then agents; order within any kind
  is not guaranteed). A relink itself succeeding is never undone by a failure in
  the `AGENTS.md` bookkeeping that follows it — `update_one` and
  `relink_candidate` both call `sync_behavior_entry` through
  `warn_on_sync_failure`, which turns its `Err` into a printed warning
  instead of propagating it, so a single candidate's bookkeeping failure
  (e.g. no resolvable entry file at the new location) never aborts the
  rest of an `--all` scan. `update_all` only attempts to load `AgentsMd`
  at all if at least one candidate is a behavior — a skill-only or
  agent-only `--all` run never requires `AGENTS.md` to exist.

## What breaks if this is touched

- Changing the two-line block format in `agents_md.rs` without updating
  both `parse_block_comment` and `serialize` breaks the round-trip
  invariant silently — there's no schema version, so an old `AGENTS.md`
  written by a previous lore version is just read as plain header text if
  the format ever changes incompatibly.
- Removing the `is_link`/`is_live` distinction (e.g. "simplifying" to one
  check) silently changes what `list` and `accounts sync` consider broken
  vs. absent.
- `list`'s account-section re-link filter (`print_dir_entries`'s
  `skip_relink_target`) depends on exact path equality with the target
  `wire::relink_skill` writes (`skills_dir.join(name)`). If either side's
  join convention changes independently, the filter silently stops
  matching — an account section would start double-listing shared
  re-links (or swallowing real account skills) with no error.
- Sorting `update --all`'s broken-candidate list (e.g. to match `list`'s
  sorted output) would change prompt order for anyone with multiple broken
  entries of the same kind — a behavior change for users mid-recovery, not
  just an internal cleanup.
- Reintroducing an unconditional exclusion of `default` from `list`'s
  account loop (instead of gating only the empty-section print on it)
  would hide any skill, behavior or agent installed via `--account default`,
  even though it's registered and wired exactly like any other account.
- Dropping `!has_agents` from the `default` silence condition would hide an
  agent added with `--account default` whenever `default` has no skills or
  behaviors of its own.
- `list`'s agent re-link filter depends on the same exact path equality as
  the skills one, with the extra wrinkle that both sides use the *raw*
  `<name>.md` file name. Stripping `.md` before the comparison (instead of
  only for display) would silently stop it matching and double-list every
  shared agent under each account.
- Sharing `symlink::is_live` between skills and agents (or making
  `is_live_file` accept directories) would misreport broken agent links: a
  link retargeted at a directory would look healthy in `list` and be skipped
  by `update --all`.
- Dropping the `scoped_agents_dir` symlink check, or turning its `bail!`
  into a warn-and-continue, lets `--account` write into or delete from a
  directory the account does not own (the shared pool, another account, a
  user folder). It must fail rather than skip: the user named one account,
  so a silent no-op would report success for an install that did not
  happen. Conversely, making the shared fan-out fail the same way would let
  one user-managed account abort re-links for all the others.
- Changing the name normalization in `agent.rs` (`strip_suffix(".md")` runs
  once, after the trailing-slash strip) changes which files a name maps to;
  `update`'s `agent_stem` is an independent copy of the same one-`.md` rule,
  so the two must be changed together.
