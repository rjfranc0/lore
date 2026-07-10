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
  `LORE.md`) — see [behaviors](behaviors.md#feature-behavior-add--remove).
- **Built-in behavior**: a behavior directory created by lore itself
  (currently only `from-claude`, see [accounts.md](../accounts.md) for the
  migration that creates it) rather than symlinked from an external repo.
  lore will not delete these automatically.
- **AGENTS.md**: the generated file at `~/.agents/AGENTS.md` that is the
  actual mechanism behaviors use to reach an agent — see
  [@/implementation/agent-config.md#agentsmd-format] for the exact format
  contract.

## Features

- [skills](skills.md) — `lore install <skill>` / `lore remove <skill>`:
  shared-by-default, `--account`-scoped symlink management for skill
  directories.
- [behaviors](behaviors.md) — `lore behavior add`/`remove`: the same
  shared/scoped model for always-on instruction directories, wired into
  `AGENTS.md` or a per-account `LORE.md`.
- [sync](sync.md) — `lore sync`: reconciles `AGENTS.md` and every
  per-account `LORE.md` against what's actually on disk.
- [list](list.md) — `lore list`: shows every installed skill/behavior,
  shared and per-account, flagging broken symlinks.
- [update](update.md) — `lore update`: re-points a skill/behavior's
  symlink (and its `AGENTS.md` entry) after its source repo has moved.

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
  repo contents. `lore update` (see [update](update.md#feature-update))
  only re-points a symlink's target path after a repo has moved on disk —
  it does not touch or sync file content either.
- No per-account **behavior update**: `lore update` still only ever
  operates on the shared `~/.agents/` tree — a scoped behavior added via
  `behavior add --account <name>` has no equivalent re-link command, it's
  only reachable via `add`/`remove` themselves. (`lore sync`'s Pass 2 *does*
  now reconcile per-account `LORE.md` entries against disk — see
  [sync](sync.md#feature-sync-agentsmd--per-account-loremd-reconciliation)
  — but that's stale-entry cleanup and missing-entry addition, not the
  target-relinking `update` does.) See [@/functional/accounts.md] for what
  else varies per account.

Both skill scoping and behavior scoping are no longer non-goals — see
[skills](skills.md#feature-skill-install--remove) and
[behaviors](behaviors.md#feature-behavior-add--remove).
