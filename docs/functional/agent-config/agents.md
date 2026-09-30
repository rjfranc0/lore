# Feature: agent add / remove

See [agent-config](index.md) for domain language (**subagent**, **shared**
vs. **scoped**) this feature builds on. It is the skill model
([skills](skills.md#feature-skill-install--remove)) applied to a single
Markdown file instead of a directory.

**What it does**: `lore agent add <name> [...]` (run from inside a repo
containing `<name>.md` files) links each named subagent either **shared**
(default, no flag) or **scoped to one Claude account** (`--account
<name>`):

- **Shared** (no `--account`): creates
  `~/.agents/agents/<name>.md → $PWD/<name>.md`, then re-links that entry
  into every registered account's `~/.claude-<account>/agents/<name>.md`
  (a re-link to the shared entry, not a second link to `$PWD`).
- **Scoped** (`--account <name>`): creates
  `~/.claude-<name>/agents/<name>.md → $PWD/<name>.md` directly, in that one
  account only — `~/.agents/agents/` and every other account are untouched.

`lore agent remove <name> [...]` mirrors this: shared remove deletes the
shared link and each account's re-link *only when that re-link still points
at the shared entry*; `--account <name>` remove deletes only that account's
link. Source files are never touched, either way.

**Why**: Claude Code discovers subagents purely from the `agents/` directory
of its config dir — nothing is registered in `AGENTS.md` or `LORE.md`, so a
symlink in the right place is the whole integration. Sharing the pool
through `~/.agents/agents/` keeps every account consistent and in sync with
the source repo, the same reason skills work this way; the `--account` scope
covers an agent that only one identity should see.

**The link keeps its `.md` extension** — Claude requires it. Only lore's
own output and `lore list` drop it (`reviewer`, not `reviewer.md`), and a
trailing `.md` typed on the command line is accepted and stripped:
`lore agent add reviewer.md` is identical to `lore agent add reviewer`. Only
one `.md` is stripped, so `reviewer.md.md` names the agent `reviewer.md`.

**`--account` targets must already be registered**: an unregistered name
fails with the same actionable error as `install --account`, and nothing is
created. **`--account default` is never implicit**: it scopes to
`~/.claude/agents/` only and does not fall back to shared behavior.

**Real files are never disturbed.** A hand-written, non-symlinked `.md`
already sitting in `~/.claude-<account>/agents/` (any account, `default`
included) is left in place and is never moved into the shared pool. If a
shared re-link would land on one, that one re-link is skipped with a
warning; the rest of the run still succeeds. The same protection covers a
scoped or shared `add` whose destination is occupied by a real file.

**A symlinked `agents/` is never written or deleted through.** When an
account's `~/.claude-<account>/agents/` is itself a symlink (the user's own
setup — it may alias the shared pool, another account, or an unrelated
directory; a dangling one counts too), lore treats the directory as
user-managed. Otherwise `--account` would stop meaning "this account only":
a scoped add could insert into the shared pool and every account would pick
it up, and a scoped remove could delete the shared link and break another
account's re-link. The two scopes react differently, on purpose:

- **Scoped** `add`/`remove --account <name>` **fail** (exit 1) with an error
  naming the symlink and asking for a real directory, before anything is
  created or removed — the user named exactly one account, so a silent no-op
  would hide the failure.
- **Shared** `add`/`remove` **warn and skip** that account's re-link or
  un-link and carry on with the others — one user-managed account must not
  abort a fan-out over many. The shared link itself is still created or
  removed as usual.

This is the same stance `init`/`accounts sync` already take toward a
symlinked `agents/` (see [@/functional/accounts.md]); a real directory
restores normal behavior.

**Acceptance conditions**:
- Given `<name>.md` exists in `$PWD`, when `lore agent add <name>` runs (no
  `--account`), then `~/.agents/agents/<name>.md` is a symlink to
  `$PWD/<name>.md`, **and** every registered account (including `default`)
  has `agents/<name>.md` resolving through the shared entry to the same file.
- Given `--account work` and a registered `work` account, when `lore agent
  add --account work <name>` runs, then only
  `~/.claude-work/agents/<name>.md` is created.
- Given `<name>.md` does not exist in `$PWD`, when `lore agent add` runs with
  several names, then lore warns for that name, skips it, installs the
  others, and exits 0.
- Given `<name>` is already linked at the targeted scope, when `lore agent
  add` runs again, then the link is not overwritten — lore warns
  `<name> already installed` and shows the existing and attempted targets
  (same format as skills).
- Given a real (non-symlink) file occupies the shared or scoped destination,
  when `lore agent add` runs, then lore warns it exists and is not a symlink
  and leaves it intact.
- Given `<name>` is not linked at the targeted scope, when `lore agent
  remove` runs, then lore warns it is not installed (naming the account for
  a scoped remove) and exits 0.
- Given an account holds a scoped link of the same name pointing at a
  different source, when a shared `lore agent remove <name>` runs, then that
  link is left in place with a warning.
- Given `~/.claude-work/agents/` is a symlink (to the shared pool, another
  account, or any other directory), when `lore agent add --account work
  <name>` or `lore agent remove --account work <name>` runs, then it exits 1
  with an error naming that path, and neither the symlink's target directory,
  the shared pool, nor any other account changes.
- Given the same symlinked `~/.claude-work/agents/`, when a shared `lore
  agent add` or `remove` runs, then lore warns that it is skipping that
  account, still creates/removes the shared link and the other accounts'
  re-links, and exits 0 — nothing is written to or deleted from the
  symlink's target.
- Given no accounts are registered, a shared add/remove only touches
  `~/.agents/agents/` — the fan-out is empty, not an error.

**Out of scope**: other tools' agent formats and directories (Codex, Gemini,
Cursor, OpenCode, Copilot), project-level `.claude/agents/`, frontmatter or
content validation, and directory-based subagent sources. There is also no
registry to reconcile, so `lore sync` does nothing for agents; a broken link
is surfaced by [list](list.md#feature-list) and repaired by
[update](update.md#feature-update).

See [@/implementation/agent-config.md#commands-built-on-these-primitives]
for how add/remove and the re-link fan-out are implemented.
