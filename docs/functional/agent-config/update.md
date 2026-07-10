# Feature: update

See [agent-config](index.md) for domain language this feature builds on.

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
non-goal in [agent-config](index.md#non-goals-this-domain) is unchanged by
this feature.
