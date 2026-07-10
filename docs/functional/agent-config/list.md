# Feature: list

See [agent-config](index.md) for domain language this feature builds on.

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
  [@/implementation/accounts/wire.md#module-wirers]), when `list` runs, then
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
