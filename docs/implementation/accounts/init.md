# Module: `commands/init.rs`

**Responsibility**: the only place that resolves *which* Claude directory a
given invocation targets, runs the migration logic, and triggers
registration.

**Validate before any disk I/O**: account name validation happens before
`LoreConfig` is even loaded, via the shared `config::validate_account_name`
free function (see [config](config.md)) — the same rule `install
--account`/`remove --account` enforce, extracted here so all three call
sites reject an invalid name identically instead of maintaining separate
copies. This ordering is itself the contract, not just a nice-to-have — a
name that's about to be rejected must leave zero trace (no directory, no
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
`wire_claude_md`'s job now (see [wire](wire.md)), invoked via
`wire_claude_dir` on *every* `init` run, for *every* account — not a
one-time thing gated on AGENTS.md's absence.

**Migration target**: just before that `wire_claude_dir` call, `init.rs`
picks which behaviors-dir/register-file pair migrated content should land
in, keyed on `account_name == "default"`: the shared `p.behaviors_dir` /
`p.agents_md` for the default account, or that account's own
`<claude_dir>/behaviors` / `wire::lore_md_path(&claude_dir)` for a named
one. This split is what keeps a named account's migrated instructions from
ever touching the shared `AGENTS.md`.

**Skill migration is gated on `account_name == "default"`** — the entire
block described below (moving real skill directories into `skills_dir`,
the collision bail) is skipped outright for a named account
(`init --account <name>`). A named account's pre-existing real `skills/`
dir is left exactly as it was; nothing from it is ever moved into the
shared pool. This is deliberate: the migration only makes sense once, for
the one account whose skills become the shared pool — running it again for
every named account would leak that account's own skills into every other
account's shared tree.

For the default account, while iterating `claude_skills`'s entries,
symlinks are skipped outright (`continue`) — only **real** (non-symlinked)
directories are candidates for migration into `skills_dir`. This is what
keeps a previous `init` run's re-links (or a scoped account-specific
symlink) untouched on every re-run: the loop only ever moves genuinely
pre-lore, unmanaged directories. Among the remaining real-directory
candidates, any name that already exists at the destination is left in
place at the source, warned about, and flagged via a `collision` bool.
After the loop, if `collision` is true the whole command bails — after
attempting every movable skill (partial progress is preserved and
reported) but before wiring `CLAUDE.md` (the command never finishes
"successfully" with conflicts still unresolved). The `claude_skills`
directory itself is never removed at the end of this step — unlike the
pre-this-feature behavior, it is now permanent infrastructure (see
[wire](wire.md)), not a symlink target to be cleared and replaced.

**A named account gets a different, softer collision path.** Since it
never migrates its own skills, a name shared between something already in
its real `skills/` dir and a skill in the shared pool is only caught later,
when `wire_claude_dir` calls `wire_claude_skills` → `relink_skill` for
every shared skill (see [wire](wire.md)): a non-symlink entry already
occupying that name makes `relink_skill` warn and skip just that one entry,
not bail the whole command. The same *kind* of collision is therefore a
hard stop for the default account but only a per-skill warning for a named
one.

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

## What breaks if this is touched

- Changing this branch back to `account.is_some()` reintroduces the
  `--account default` collision: a fully-wired, registry-invisible
  `~/.claude-default/` directory.
- Removing the `account_name == "default"` gate on skill migration would
  make `init --account <name>` start moving a named account's own real
  skills into the shared pool again — leaking them into every other
  account instead of leaving that account's `skills/` dir alone.
