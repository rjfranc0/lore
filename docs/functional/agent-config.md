# Agent Config

## What this covers

lore's original surface: a single directory (`~/.agents/`) that holds AI agent
skills and behaviors as symlinks into the repos that actually own them, plus
one generated file (`AGENTS.md`) that universal agent tooling reads at
session start.

## Domain language

- **Skill**: a self-contained capability directory (expected to contain a
  `SKILL.md`) that an agent loads on demand. lore never copies a skill's
  contents — it only manages a symlink at `~/.agents/skills/<name>` pointing
  at the skill's real location in some repo.
- **Behavior**: a directory of standing instructions (an entry `.md` file —
  see resolution order below) that an agent should load at the *start of
  every session*, not on demand. The distinction from a skill is exactly
  this: skills are pulled in when needed, behaviors are always-on.
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
containing skill directories) creates a symlink
`~/.agents/skills/<skill> → $PWD/<skill>` for each name given. `lore remove
<skill> [...]` deletes that symlink only — the source directory in its repo
is never touched.

**Why**: skills should be usable across every agent/tool without copying
files into N different config directories, and should stay perfectly in sync
with their source repo (pull the repo, the skill updates — no separate
"update" step needed for content changes, see Non-goals).

**Acceptance conditions**:
- Given a directory `<skill>/` exists in `$PWD`, when `lore install <skill>`
  runs, then `~/.agents/skills/<skill>` exists as a symlink to
  `$PWD/<skill>`.
- Given `<skill>` is already installed (symlink exists), when `lore install
  <skill>` runs again, then lore does not overwrite it — it warns and shows
  both the existing target and the attempted one, so a name collision
  between two different source repos is visible rather than silently
  resolved.
- Given `<skill>` is not installed, when `lore remove <skill>` runs, then
  lore warns "`<skill>` is not installed" and exits 0 (removal of a
  non-existent thing is not an error).
- A trailing slash on the name (`lore install my-skill/`, common from shell
  tab-completion) is stripped before use — `my-skill` and `my-skill/` are
  the same skill.

**Example** (name collision across two source repos — verified against the
built binary):
```
⚠  cooking-chef already installed
  existing  → /old/repo/skills/cooking-chef
  attempted → /new/repo/skills/cooking-chef
```

**Out of scope**: lore does not validate the *contents* of a skill directory
(e.g. that `SKILL.md` exists) at install time — `lore list` is the only
place a broken symlink becomes visible.

## Feature: behavior add / remove

**What it does**: `lore behavior add <name> [...]` does three things per
name, in order: (1) symlinks `~/.agents/behaviors/<name> → $PWD/<name>`, (2)
locates that behavior's entry file, (3) appends a two-line block referencing
it to `AGENTS.md`. `lore behavior remove <name> [...]` reverses both —
deletes the symlink and strips the matching block from `AGENTS.md`.

**Why**: a skill is meaningless to an agent until something tells the agent
to load it — behaviors are that "something" for always-on instructions
specifically (as opposed to skills, which agents discover and load
themselves).

**Entry file resolution** (which file inside the behavior directory becomes
the `@import` target): `RULES.md` → `BEHAVIOR.md` → `README.md` → first
`.md` file alphabetically. If none exist, `add` fails outright — a behavior
with no resolvable entry point cannot be wired into `AGENTS.md` at all.

**Acceptance conditions**:
- Requires `lore init` to have already run (`AGENTS.md` must exist) —
  `behavior add`/`remove` fail immediately with "Run 'lore init' first"
  otherwise. This is a precondition, not a recoverable warning.
- Given a behavior is already symlinked, when `add` runs again, then the
  symlink step is skipped (idempotent) but the AGENTS.md-membership check
  still runs independently — each of the two steps (symlink, AGENTS.md
  entry) is idempotent on its own, not just the command as a whole.
  > ⚠️ **Inferred:** this independence looks deliberate (it lets a
  > half-completed prior run — symlink created but AGENTS.md write failed —
  > self-heal on retry) but is not stated anywhere explicitly; verify with
  > the original author if this matters for a future change.
- Removal matches by **exact behavior name**, not prefix/pattern — removing
  `a.c` must never also remove `axc`. This is enforced at the parser level,
  see [@/implementation/agent-config.md#agentsmd-format].
- **Built-in behaviors cannot be removed automatically.** `from-claude` is a
  real directory, not a symlink — `behavior remove from-claude` detects this
  and prints the exact manual commands instead of acting, rather than
  guessing whether deleting a non-symlinked directory is safe.

**Example** (verified against the built binary):
```
⚠  from-claude is a built-in behavior — remove manually:
  rm -rf /home/you/.agents/behaviors/from-claude
  Then remove its <!-- from-claude --> block from /home/you/.agents/AGENTS.md
```

## Feature: sync (AGENTS.md reconciliation)

**What it does**: `lore sync` reconciles `AGENTS.md` against whatever is
actually present in `~/.agents/behaviors/` on disk — removing entries whose
directory is gone, adding entries for directories present but unregistered,
leaving correct entries untouched (no reordering, no duplication).

**Why**: behaviors can be added/removed/restructured directly on disk (e.g.
splitting one behavior into several, or manually deleting a directory)
without going through `behavior add`/`remove` — `sync` is the recovery path
that brings `AGENTS.md` back in line without a human having to hand-edit a
file lore considers its own.

**Acceptance conditions**:
- Given `AGENTS.md` references a behavior whose directory no longer exists
  in `~/.agents/behaviors/`, when `sync` runs, then that entry is removed
  and reported.
- Given a directory exists in `~/.agents/behaviors/` with no corresponding
  `AGENTS.md` entry, when `sync` runs, then an entry is added (via the same
  entry-file resolution order as `behavior add`) and reported.
- Given nothing is out of sync, when `sync` runs, then it prints "AGENTS.md
  already in sync" and writes nothing (no spurious file touch / mtime
  change).
- Same precondition as `behavior add`: requires `lore init` to have run
  first.

**Out of scope**: `sync` only reconciles `AGENTS.md` against *behaviors*. It
has no opinion on skills, and (since the accounts feature) no opinion on
Claude account wiring — that reconciliation is a separate command, `accounts
sync`, see [@/functional/accounts.md#feature-accounts-sync].

## Feature: list

**What it does**: `lore list` prints every skill and behavior currently
wired, each with its symlink target. A broken symlink (target no longer
exists) is suffixed `✗ broken` rather than hidden. A built-in (non-symlinked)
behavior directory is suffixed `(migrated)` under Skills or `(built-in)`
under Behaviors instead of a target path.

**Why**: symlink staleness is invisible from a normal glance — `list` is the
one command that surfaces rot (broken targets) without requiring the user to
manually resolve every symlink.

**Acceptance conditions**:
- Given no skills are installed, when `list` runs, then the Skills section
  prints `(none)` rather than an empty list with no explanation.
- Given a skill/behavior symlink's target directory has been deleted or
  moved, when `list` runs, then that entry is still shown (not silently
  skipped) with the `✗ broken` marker — broken links are exactly the thing
  this command exists to surface, see
  [@/implementation/agent-config.md#module-symlinkrs] for the liveness check
  itself.

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
- No per-account skill or behavior scoping — every Claude account wired by
  `init` shares the exact same `~/.agents/skills/` and `behaviors/`. See
  [@/functional/accounts.md] for what *does* vary per account.
