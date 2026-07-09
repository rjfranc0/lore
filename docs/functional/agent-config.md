# Agent Config

## What this covers

lore's original surface: a single directory (`~/.agents/`) that holds AI agent
skills and behaviors as symlinks into the repos that actually own them, plus
one generated file (`AGENTS.md`) that universal agent tooling reads at
session start.

## Domain language

- **Skill**: a self-contained capability directory (expected to contain a
  `SKILL.md`) that an agent loads on demand. lore never copies a skill's
  contents — it only manages a symlink, either shared
  (`~/.agents/skills/<name>`, re-linked into every account) or scoped to
  one account (`~/.claude-<account>/skills/<name>` directly), pointing at
  the skill's real location in some repo.
- **Behavior**: a directory of standing instructions (an entry `.md` file —
  see resolution order below) that an agent should load at the *start of
  every session*, not on demand. The distinction from a skill is exactly
  this: skills are pulled in when needed, behaviors are always-on. Like
  skills, a behavior can be **shared** (symlinked into
  `~/.agents/behaviors/`, registered in `AGENTS.md`) or **scoped to one
  Claude account** (`--account <name>`, symlinked into
  `~/.claude-<name>/behaviors/`, registered in that account's own
  `LORE.md`) — see [Feature: behavior add / remove](#feature-behavior-add--remove).
- **Built-in behavior**: a behavior directory created by lore itself
  (currently only `from-claude`, see [accounts.md](accounts.md) for the
  migration that creates it) rather than symlinked from an external repo.
  lore will not delete these automatically.
- **AGENTS.md**: the generated file at `~/.agents/AGENTS.md` that is the
  actual mechanism behaviors use to reach an agent — see
  [@/implementation/agent-config.md#agentsmd-format] for the exact format
  contract.

## Feature: skill install / remove

**What it does**: `lore install <skill> [...]` (run from inside a repo
containing skill directories) installs each name either **shared**
(default, no flag) or **scoped to one Claude account** (`--account
<name>`):

- **Shared** (no `--account`): creates
  `~/.agents/skills/<skill> → $PWD/<skill>`, then re-links that same skill
  into every registered account's `~/.claude-<account>/skills/<skill>` (a
  re-link, not a fresh symlink to `$PWD` — see
  [@/functional/accounts.md#feature-lore-init---account-name] for why the
  account skills dir can hold both kinds side by side).
- **Scoped** (`--account <name>`): creates
  `~/.claude-<name>/skills/<skill> → $PWD/<skill>` directly, in that one
  account only — `~/.agents/skills/` and every other account are left
  completely untouched.

`lore remove <skill> [...]` mirrors this: shared remove deletes the shared
symlink and the re-link from every registered account; `--account <name>`
remove deletes only that one account's link. Source directories in their
repos are never touched, either way.

**Why**: the same skill (a shell utility, a domain-specific agent skill)
often needs to be visible everywhere, but some skills only make sense for
one identity (e.g. a work-only tool) — shared-by-default with an opt-in
account scope covers both without a second command. Skills should stay in
sync with their source repo (pull the repo, the skill updates — no
separate "update" step needed for content changes, see Non-goals).

**`--account` targets must already be registered**: `lore install --account
ghost <skill>` (an account never wired via `lore init --account ghost`)
fails with an actionable error and creates nothing — an unregistered name
is treated as a likely typo, not an implicit "wire it now."

**The `default` account is never implicit**: `lore install --account
default <skill>` scopes to `~/.claude/skills/` only, exactly like any other
named account — it does not fall back to the shared, all-accounts behavior
just because its target happens to be the same directory `init` (no flag)
also wires. Only omitting `--account` entirely triggers the shared
behavior.

**Acceptance conditions**:
- Given a directory `<skill>/` exists in `$PWD`, when `lore install <skill>`
  runs (no `--account`), then `~/.agents/skills/<skill>` exists as a symlink
  to `$PWD/<skill>`, **and** every registered account's
  `~/.claude-<account>/skills/<skill>` exists as a re-link resolving to the
  same target.
- Given `--account work` and a registered `work` account, when `lore
  install --account work <skill>` runs, then only
  `~/.claude-work/skills/<skill>` is created — `~/.agents/skills/` and
  every other account are unaffected.
- Given `<skill>` is already installed at the targeted scope (shared or a
  specific account), when `lore install` runs again, then lore does not
  overwrite it — it warns and shows both the existing target and the
  attempted one, so a name collision is visible rather than silently
  resolved.
- Given `<skill>` is not installed at the targeted scope, when `lore
  remove` runs, then lore warns it isn't installed (naming the account for
  a scoped remove) and exits 0 (removal of a non-existent thing is not an
  error).
- A trailing slash on the name (`lore install my-skill/`, common from shell
  tab-completion) is stripped before use, for both the shared and
  `--account` paths.
- No registered accounts exist yet: a shared install/remove only touches
  `~/.agents/skills/` — the account fan-out loop is simply empty, not an
  error.

**Example** (name collision across two source repos — verified against the
built binary):
```
⚠  cooking-chef already installed
  existing  → /old/repo/skills/cooking-chef
  attempted → /new/repo/skills/cooking-chef
```

**Out of scope**: lore does not validate the *contents* of a skill directory
(e.g. that `SKILL.md` exists) at install time — `lore list` is the only
place a broken symlink becomes visible. This feature only changes what
`install`/`remove` do; see [Feature: list](#feature-list) for how `list`
surfaces shared vs. per-account skills.

## Feature: behavior add / remove

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
[@/implementation/accounts.md#module-wirers]). `~/.agents/AGENTS.md` and
every other account are left completely untouched — mirrors the
shared-by-default/opt-in-scoped shape of [skill install/remove](#feature-skill-install--remove),
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

## Feature: sync (AGENTS.md + per-account LORE.md reconciliation)

**What it does**: `lore sync` runs two passes. Pass 1 reconciles
`AGENTS.md` against whatever is actually present in `~/.agents/behaviors/`
on disk — removing entries whose directory is gone, adding entries for
directories present but unregistered. Pass 2 does the same reconciliation
for every registered account's own `LORE.md` against
`~/.claude-<account>/behaviors/`, and additionally re-adds the
`@~/.agents/AGENTS.md` import line as `LORE.md`'s first line if it's
missing or has drifted. Both passes leave correct entries untouched (no
reordering, no duplication).

**Why**: behaviors can be added/removed/restructured directly on disk (e.g.
splitting one behavior into several, or manually deleting a directory)
without going through `behavior add`/`remove` — `sync` is the recovery path
that brings `AGENTS.md` and every account's `LORE.md` back in line without
a human having to hand-edit a file lore considers its own. Per-account
`LORE.md` can drift the same way shared `AGENTS.md` can — a scoped
behavior's symlink can go stale, or a user can hand-edit `LORE.md` and
strip its `AGENTS.md` import — so it gets the same self-healing treatment,
via a shared `reconcile_behaviors` helper both passes call.

**Acceptance conditions**:
- Given `AGENTS.md` references a behavior whose directory no longer exists
  in `~/.agents/behaviors/`, when `sync` runs, then that entry is removed
  and reported.
- Given a directory exists in `~/.agents/behaviors/` with no corresponding
  `AGENTS.md` entry, when `sync` runs, then an entry is added (via the same
  entry-file resolution order as `behavior add`) and reported.
- Given an account's `LORE.md` references a behavior whose directory no
  longer exists in `~/.claude-<account>/behaviors/`, when `sync` runs, then
  that entry is removed and reported as `Removed stale entry from
  <account>: <name>`.
- Given a directory exists in `~/.claude-<account>/behaviors/` with no
  corresponding `LORE.md` entry, when `sync` runs, then an entry is added
  and reported as `Added <account>/<name> to LORE.md`.
- Given an account's `LORE.md` is missing its `@~/.agents/AGENTS.md` import
  as the first line (stripped or replaced by hand), when `sync` runs, then
  the header is restored and reported.
- Given an account's `LORE.md` file itself doesn't exist, when `sync` runs,
  then that account is skipped with a warning (not a failure) pointing at
  `lore init --account <name>`, and the rest of the run (AGENTS.md pass,
  other accounts) still completes.
- Given nothing is out of sync anywhere, when `sync` runs, then it prints a
  single "✓ Already in sync" and writes nothing — regardless of how many
  accounts are registered.
- Given something changed somewhere but not everywhere, when `sync` runs,
  then it emits granular lines for the untouched targets — "AGENTS.md
  already in sync" and "account: `<name>` — already in sync" per unchanged
  account.
- Same precondition as `behavior add`: requires `lore init` to have run
  first (checked once, against the shared `AGENTS.md`).

**Out of scope**: `sync` reconciles `AGENTS.md` and per-account `LORE.md`
content — *behavior entries and the AGENTS.md import line* — against what's
on disk. It has no opinion on skills, and no opinion on the rest of Claude
account wiring (`CLAUDE.md`'s import of `LORE.md`, the skills directory
shape) — that's a separate command, `accounts sync`, see
[@/functional/accounts.md#feature-accounts-sync]. The two commands can
legitimately both touch `LORE.md`: `accounts sync` only checks/repairs
whether `LORE.md` exists and imports `AGENTS.md` as part of the wiring
chain; `lore sync`'s Pass 2 additionally reconciles the *behavior entries*
inside it.

## Feature: list

**What it does**: `lore list` prints `Shared skills:` and `Shared
behaviors:` first — every skill/behavior wired into the shared
`~/.agents/` tree, each with its symlink target. A broken symlink (target
no longer exists) is suffixed `✗ broken` rather than hidden. A built-in
(non-symlinked) behavior directory is suffixed `(migrated)` under Skills or
`(built-in)` under Behaviors instead of a target path. After the shared
sections, one `Account: <name>` section is printed per registered account
other than `default` (in `lore.toml`'s alphabetical `[accounts]` order),
each with its own `Skills:`/`Behaviors:` sub-listing scoped to that
account's own directories — same broken/migrated/built-in labeling rules
apply within a section.

**Why**: symlink staleness is invisible from a normal glance — `list` is the
one command that surfaces rot (broken targets) without requiring the user to
manually resolve every symlink. The per-account breakdown exists for the
same reason multi-account scoping does at all (see
[@/functional/accounts.md]): once a skill or behavior can be installed into
one account only, a flat listing can no longer answer "what does account X
actually have" — this feature makes that question answerable directly.

**Acceptance conditions**:
- Given no skills are installed, when `list` runs, then the Skills section
  prints `(none)` rather than an empty list with no explanation. Same rule
  applies per-account: an empty `Skills:`/`Behaviors:` sub-section under an
  `Account:` header still prints `(none)` rather than being omitted — this
  is what distinguishes "account is registered but empty" from "account
  doesn't exist."
- Given a skill/behavior symlink's target directory has been deleted or
  moved, when `list` runs, then that entry is still shown (not silently
  skipped) with the `✗ broken` marker — broken links are exactly the thing
  this command exists to surface, see
  [@/implementation/agent-config.md#module-symlinkrs] for the liveness check
  itself. Applies identically inside an `Account:` section.
- Given no accounts are registered beyond `default`, when `list` runs, then
  no `Account:` section is printed at all — output is exactly the two
  shared sections, matching the pre-multi-account output shape.
- Given the `default` account has its own scoped installs (e.g. `lore
  install --account default <skill>`), when `list` runs, then an `Account:
  default` section **is** printed, scoped exactly like any other account's
  section — `default` only stays silent when it has nothing
  account-specific to show.
- Given an account has a skill installed via the shared path (fanned out as
  a re-link into that account's skills dir, see
  [@/implementation/accounts.md#module-wirers]), when `list` runs, then
  that re-link is **not** repeated under the account's own `Skills:`
  section — it already appears once, under `Shared skills:`. Only skills
  installed directly into that account (`lore install --account <name>`)
  appear under its own section.

**Example**:
```
Skills:
  cooking-chef             → /home/you/repos/ai-restaurant/skills/cooking-chef
  broken-skill             → /old/path/that/is/gone  ✗ broken

Behaviors:
  from-claude                (built-in)
  restaurant-rules         → /home/you/repos/ai-restaurant/behaviors/restaurant-rules
```

## Feature: update

**What it does**: `lore update <name>` re-links an existing skill or
behavior's symlink to a new source location — `~/.agents/skills/<name>` or
`~/.agents/behaviors/<name>` is force-relinked to `$PWD/<name>` (or to
`--path <path>` if given), regardless of whether the existing symlink was
already healthy or broken. For a behavior, the matching `AGENTS.md`
block's `@path` line is also re-resolved and rewritten if the entry
filename changed (e.g. the new location uses `README.md` where the old
one used `RULES.md`). `lore update --all` scans both
`~/.agents/skills/` and `~/.agents/behaviors/` for broken symlinks and
prompts for a replacement path per entry, one at a time.

**Why**: moving or renaming a repo on disk breaks every symlink lore
created into it. Before `update`, recovering meant manually `remove`-ing
and re-`install`-ing (or re-`behavior add`-ing) each one by hand, including
re-deriving the `AGENTS.md` entry for behaviors by hand too.

**Acceptance conditions**:
- Given `<name>` is installed as either a skill or a behavior (skills are
  checked first if a name could theoretically match both), when `lore
  update <name>` runs from inside the new source directory, then the
  symlink is replaced to point at `$PWD/<name>`.
- Given `--path <new-path>` is supplied, when `lore update <name>` runs,
  then `<new-path>` is used as the new symlink target instead of
  `$PWD/<name>` — the command does not need to run from inside the new
  source directory at all.
- Given `<name>`'s symlink is already healthy, when `lore update <name>`
  runs, then it still relinks unconditionally to the new target — `update`
  is a force operation, not a "repair only if broken" one.
- Given `<name>` is a behavior whose new location's resolved entry file
  differs from what's currently recorded in `AGENTS.md`, when `lore update
  <name>` runs, then the `@path` line is rewritten to the new entry file;
  if the resolved entry file is unchanged, `AGENTS.md` is left untouched.
- Given `<name>` is not installed as either a skill or a behavior, when
  `lore update <name>` runs, then it fails with `'<name>' is not installed
  as a skill or behavior`.
- Given no broken symlinks exist in either `~/.agents/skills/` or
  `~/.agents/behaviors/`, when `lore update --all` runs, then it prints
  "No broken symlinks found" and exits 0 — no prompts.
- Given one or more broken symlinks exist, when `lore update --all` runs,
  then each is shown with its dead target and prompted for a replacement
  path; a blank answer skips that entry (reported, scan continues) and a
  non-directory answer warns and skips it too — one bad answer never
  aborts the rest of the scan.
- Given a candidate relinks successfully but the subsequent `AGENTS.md`
  bookkeeping step then fails (e.g. a behavior's new location has no
  resolvable entry file, or `AGENTS.md` itself doesn't exist), when `lore
  update`/`lore update --all` runs, then the relink stands — only the
  `AGENTS.md` update is skipped, reported as a warning naming the
  behavior; for `--all`, the scan continues to the next candidate exactly
  as it does for a skipped blank/non-directory answer.
- Given neither `<name>` nor `--all` is given (or both are given), when
  `lore update` runs, then it fails with a clear error rather than
  guessing intent.
- In every case above, the source directory's files and content are never
  read, written, or deleted — only the symlink and, for behaviors, the
  `AGENTS.md` entry pointing at it, change. See
  [@/implementation/agent-config.md#commands-built-on-these-primitives]
  for how relinking and the `AGENTS.md` resync are implemented.

**Example** (skill moved, behavior's entry filename changed — verified
against the built binary):
```
✓ Relinked cooking-chef → /new/repo/skills/cooking-chef
✓ Relinked restaurant-rules → /new/repo/behaviors/restaurant-rules
✓ Updated AGENTS.md entry for restaurant-rules → /home/you/.agents/behaviors/restaurant-rules/README.md
```

**Example** (new location has no resolvable entry file — relink still
completes, only the `AGENTS.md` update is skipped; verified against the
built binary):
```
✓ Relinked my-rules → /new/repo/no-entry-here
⚠  Could not update AGENTS.md entry for my-rules: no .md entry point found in /home/you/.agents/behaviors/my-rules
```

**Out of scope**: `update` only ever changes *where a symlink points*. It
never reads, diffs, or copies file content — the "no content-sync command"
non-goal below is unchanged by this feature.

## Files

| Path | Description |
|---|---|
| `~/.agents/AGENTS.md` | Universal agent instructions — format detailed in [@/implementation/agent-config.md#agentsmd-format] |
| `~/.agents/skills/` | Skill symlinks |
| `~/.agents/behaviors/` | Behavior symlinks and built-ins |

## Non-goals (this domain)

- No "update" command for skill/behavior *content* — content always lives
  in the source repo and is read live through the symlink; `lore
  install`/`add` only ever (re)point a symlink, they never pull or sync
  repo contents. `lore update` (see [Feature: update](#feature-update))
  only re-points a symlink's target path after a repo has moved on disk —
  it does not touch or sync file content either.
- No per-account **behavior update**: `lore update` still only ever
  operates on the shared `~/.agents/` tree — a scoped behavior added via
  `behavior add --account <name>` has no equivalent re-link command, it's
  only reachable via `add`/`remove` themselves. (`lore sync`'s Pass 2 *does*
  now reconcile per-account `LORE.md` entries against disk — see
  [Feature: sync](#feature-sync-agentsmd--per-account-loremd-reconciliation)
  — but that's stale-entry cleanup and missing-entry addition, not the
  target-relinking `update` does.) See [@/functional/accounts.md] for what
  else varies per account.
