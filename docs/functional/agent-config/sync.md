# Feature: sync (AGENTS.md + per-account LORE.md reconciliation)

See [agent-config](index.md) for domain language this feature builds on.

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
