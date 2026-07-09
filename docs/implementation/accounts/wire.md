# Module: `wire.rs`

**Responsibility**: the only place that knows the on-disk layout of a
Claude config directory, and the only place that actually writes one.

```rust
pub fn claude_md_path(claude_dir: &Path) -> PathBuf        // claude_dir.join("CLAUDE.md")
pub fn lore_md_path(claude_dir: &Path) -> PathBuf           // claude_dir.join("LORE.md")
pub fn claude_skills_path(claude_dir: &Path) -> PathBuf     // claude_dir.join("skills")
pub fn claude_behaviors_path(claude_dir: &Path) -> PathBuf  // claude_dir.join("behaviors")

pub fn wire_lore_md(agents_md: &Path, claude_dir: &Path) -> Result<PathBuf>
pub fn wire_claude_md(claude_dir: &Path, agents_md: &Path,
                       migration_behaviors_dir: &Path, migration_register_md: &Path) -> Result<()>
pub fn wire_claude_skills(skills_dir: &Path, claude_dir: &Path) -> Result<()>
pub fn relink_skill(skills_dir: &Path, claude_dir: &Path, name: &str) -> Result<()>
pub fn unlink_account_skill(claude_dir: &Path, name: &str) -> Result<()>
pub fn wire_claude_dir(agents_md: &Path, skills_dir: &Path, claude_dir: &Path,
                        migration_behaviors_dir: &Path, migration_register_md: &Path) -> Result<()>
```

**`LORE.md` is lore's fully-owned intermediary** between the universal
`AGENTS.md` and the shared, partly user-owned `CLAUDE.md`. `wire_lore_md`
reuses the `AgentsMd` struct as-is (its header + named-block format is
byte-identical to what `LORE.md` needs) — loads the existing file if
present, **unconditionally overwrites just the header** to
`@{agents_md}\n`, and saves. The unconditional overwrite is what makes it
idempotent without a separate "is it already correct" branch — nothing
foreign can ever land in `LORE.md`'s header, so there's nothing to
preserve there. `md.behaviors` (any blocks added by a prior
`behavior add --account`-style registration) is left untouched.

**Self-heals on an unreadable `LORE.md`.** If `AgentsMd::load` fails (a
non-UTF-8 file, a permission error) `wire_lore_md` does not propagate the
error — it warns (`"{path} is unreadable ({e}) — recreating it"`) and
falls back to `AgentsMd::parse("")`, i.e. treats it as if the file didn't
exist, discarding whatever couldn't be read. This is a deliberate fix, not
incidental: `accounts sync`/`init` previously *claimed* to self-heal an
unreadable `LORE.md`/`CLAUDE.md` (see [commands](commands.md)) but
actually aborted with the read error, because the rewire path went through
this same load call. `wire_claude_md` has the matching behavior for
`CLAUDE.md` — a read failure there is caught explicitly (not via `?`) and
logged with the same "unreadable — replacing" wording, then treated as
`content = None`, which falls through to the "absent" case (whole file
becomes the `LORE.md` import) rather than migrating unreadable bytes.

**`wire_claude_md` never fully overwrites `CLAUDE.md`.** It normalizes the
path first (a symlink or directory sitting where CLAUDE.md should be is
removed, exactly as the old `wire_claude_dir` did), reads whatever content
remains (`None` if absent or unreadable — see the self-heal note above),
then applies this priority order — order matters, each case returns before
the next is checked:

1. A line already equals (trimmed) `@{lore_md}` → already wired, no-op.
2. A line already equals (trimmed) `@{agents_md}` (the legacy pre-LORE.md
   direct import) → that one line is replaced with `@{lore_md}`, nothing
   else in the file is touched.
3. Content exists and is non-empty after trimming → migrate (see below).
4. Otherwise (absent, or present but empty) → the entire file becomes
   `@{lore_md}\n`.

**Migration** (private `migrate_claude_md`) copies the *original* content
verbatim into `migration_behaviors_dir/from-claude/RULES.md`, registers a
`from-claude` block into `migration_register_md` (guarded by
`contains_name` so a retry never double-registers), then appends
`@{lore_md}` to that same original content and writes the result back —
the old text is never deleted, only added to. It also reports every line
in the original content that (trimmed) starts with `@`: those belong to
other tools that already write their own lines into `CLAUDE.md`, and are
named in the migration warning specifically so the user knows lore saw
them and left them alone.

**`migration_behaviors_dir`/`migration_register_md` are caller-supplied,
not derived here** — `wire_claude_md` doesn't know or care which account
it's wiring. For the default account the caller passes the shared
`~/.agents/behaviors/` and `~/.agents/AGENTS.md`; for a named account it
passes that account's own `<claude_dir>/behaviors/` and that account's own
`LORE.md` — see [init](init.md) for where that split happens.

**`wire_claude_skills` produces a real directory, not a symlink** — this
changed from the original single-symlink model (see
[@/functional/accounts.md#feature-lore-init---account-name]): it removes a
stale legacy symlink if one sits at `claude_dir/skills` (the old model),
`create_dir_all`s the directory, then re-links every entry currently in
`skills_dir` into it via `relink_skill`. It never wipes the directory on a
re-run — any account-specific symlink already living there (from a scoped
`install --account`) survives every subsequent `init`/`sync` call, because
the directory is only ever added to, never rebuilt from scratch.

**`relink_skill(skills_dir, claude_dir, name)`** re-links one shared skill
into one account: `create_dir_all`s the account's skills dir defensively,
then creates `claude_dir/skills/<name> → skills_dir/<name>` only if
nothing is already linked there. Create-if-absent, never overwrite — this
is what makes both `wire_claude_skills`'s full re-link loop and a single
`install` call safe to run repeatedly. If a non-symlink entry already
occupies that path — manual tampering, since lore itself never places one
there post-init — `relink_skill` warns (naming the skill and the account's
skills path) and skips rather than letting the raw `EEXIST` from
`symlink::create` fail the whole fan-out over one account's collision.

**`unlink_account_skill(claude_dir, skills_dir, name)`** removes one
account's link for `name`, but only if it actually resolves to
`skills_dir/<name>` — i.e., it's a genuine re-link of the shared skill.
An account-scoped install of the same name pointing at a different source
(reachable per the scoped-then-shared collision case above) is left in
place with a `warn()` rather than destroyed. Silent no-op if no symlink is
present at all. Used by `remove` (shared path) to fan out the un-link
across every registered account without needing to know in advance which
accounts actually have that skill linked as a re-link (versus scoped).

Both functions are re-link **primitives** — `install`/`remove` (see
[@/implementation/agent-config.md#commands-built-on-these-primitives]) are
the callers that decide *which* accounts to loop over and whether the
operation is shared or scoped to one name.

`wire_claude_dir` is the orchestrator, in a fixed order:
`create_dir_all(claude_dir)` → `wire_lore_md` → `wire_claude_md` →
`wire_claude_skills`. The order is load-bearing: `LORE.md` must exist
before `wire_claude_md` runs, because cases 1/2/4 above all write a line
that names it, and a Case-3 migration for a named account also registers
into that same freshly-ensured `LORE.md`.

**`claude_md_path`/`lore_md_path`/`claude_skills_path`/`claude_behaviors_path`
exist as the single source of truth for those joins** — every caller that
needs to know where a Claude account's `CLAUDE.md`, `LORE.md`, skills
symlink, or scoped-behaviors directory lives ([init](init.md),
[commands](commands.md), `commands/behavior.rs`'s scoped `add`/`remove`,
and `wire.rs` itself) calls through these four functions rather than
independently writing `claude_dir.join(...)`. This was a deliberate
de-duplication: the layout rule used to be computed in multiple places
independently, which is a correctness risk (multiple places to keep in
sync, not just lines to keep short) — see "What breaks if this is
touched," below. `claude_behaviors_path` was added alongside
`commands/behavior.rs`'s scoped `add_scoped`/`remove_scoped`, replacing an
inline `claude_dir.join("behaviors")` that `init.rs`'s migration-target
tuple used to compute independently (see [init](init.md)).

## What breaks if this is touched

- Reverting the `claude_md_path`/`claude_skills_path`/`claude_behaviors_path`
  centralization (going back to inline `.join(...)` calls in
  [init](init.md)/[commands](commands.md)/`commands/behavior.rs`)
  reintroduces the duplicated-knowledge risk these helpers were added to
  close — a future layout change would again need to be applied in
  multiple places by hand.
- Calling `wire_claude_md` before `wire_lore_md` inside `wire_claude_dir`
  breaks every case that writes a `@{lore_md}` line — `LORE.md` wouldn't
  exist yet at the path being named, and a Case-3 migration for a named
  account would have nothing to register into.
- Reverting `wire_claude_skills` back to a single symlink (instead of a
  real directory of re-links) breaks per-account skill scoping outright —
  there would be nowhere for an account-specific symlink to live alongside
  the shared re-links. It would also desync `accounts sync`'s wired-check
  (see [commands](commands.md)), which now explicitly expects a real,
  non-symlinked directory.
- Replacing `wire_lore_md`/`wire_claude_md`'s explicit-catch-and-warn read
  handling with a plain `?` (e.g. during a refactor that looks like
  dead-code cleanup) silently reintroduces the bug fixed by making `sync`
  self-heal an unreadable `LORE.md`/`CLAUDE.md` — the rewire path would
  fail outright on the exact files it exists to repair.
