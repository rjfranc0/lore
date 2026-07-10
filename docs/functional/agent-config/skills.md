# Feature: skill install / remove

See [agent-config](index.md) for domain language (**skill**, **shared** vs.
**scoped**) this feature builds on.

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
separate "update" step needed for content changes, see [agent-config
Non-goals](index.md#non-goals-this-domain)).

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
`install`/`remove` do; see [list](list.md#feature-list) for how `list`
surfaces shared vs. per-account skills.
