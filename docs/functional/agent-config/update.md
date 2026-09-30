# Feature: update

See [agent-config](index.md) for domain language this feature builds on.

**What it does**: `lore update <name>` re-links an existing skill,
behavior or agent's symlink to a new source location —
`~/.agents/skills/<name>`, `~/.agents/behaviors/<name>` or
`~/.agents/agents/<name>.md` is force-relinked to `$PWD/<name>` (or
`$PWD/<name>.md` for an agent, or to `--path <path>` if given), regardless
of whether the existing symlink was already healthy or broken. For a
behavior, the matching `AGENTS.md` block's `@path` line is also re-resolved
and rewritten if the entry filename changed (e.g. the new location uses
`README.md` where the old one used `RULES.md`); agents have no `AGENTS.md`
entry, so only the link changes. `lore update --all` scans
`~/.agents/skills/`, `~/.agents/behaviors/` and `~/.agents/agents/` for
broken symlinks and prompts for a replacement path per entry, one at a time
(skills first, then behaviors, then agents).

**Agents differ in shape, not in rule.** An agent's source is a file, so
`--path` and the `--all` prompt answer must name an existing *file*
(everything else must name a directory), and `<name>` may be given with or
without its `.md` (`lore update reviewer.md` = `lore update reviewer`).
Agents are shown by name without `.md` in the `--all` prompt. The account
re-links need no update: they point at the shared link, so re-pointing it
moves every account at once.

**Why**: moving or renaming a repo on disk breaks every symlink lore
created into it. Before `update`, recovering meant manually `remove`-ing
and re-`install`-ing (or re-`behavior add`-ing) each one by hand, including
re-deriving the `AGENTS.md` entry for behaviors by hand too.

**Acceptance conditions**:
- Given `<name>` is installed as a skill, behavior or agent (looked up in
  that order if a name could match more than one type), when `lore
  update <name>` runs from inside the new source directory, then the
  symlink is replaced to point at `$PWD/<name>` (`$PWD/<name>.md` for an
  agent).
- Given `<name>` is an agent and `$PWD/<name>.md` (or the `--path` value) is
  not an existing file, when `lore update <name>` runs, then it fails with
  `'<path>' not found` and the existing link is left as it was.
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
- Given `<name>` is not installed as a skill, behavior or agent, when
  `lore update <name>` runs, then it fails with `'<name>' is not installed
  as a skill, behavior, or agent`.
- Given no broken symlinks exist in `~/.agents/skills/`,
  `~/.agents/behaviors/` or `~/.agents/agents/`, when `lore update --all`
  runs, then it prints "No broken symlinks found" and exits 0 — no prompts.
- Given one or more broken symlinks exist, when `lore update --all` runs,
  then each is shown with its dead target and prompted for a replacement
  path; a blank answer skips that entry (reported, scan continues) and an
  invalid answer (a non-directory for a skill/behavior, a non-file for an
  agent) warns and skips it too — one bad answer never aborts the rest of
  the scan.
- Given a candidate relinks successfully but the subsequent `AGENTS.md`
  bookkeeping step then fails (e.g. a behavior's new location has no
  resolvable entry file, or `AGENTS.md` itself doesn't exist), when `lore
  update`/`lore update --all` runs, then the relink stands — only the
  `AGENTS.md` update is skipped, reported as a warning naming the
  behavior; for `--all`, the scan continues to the next candidate exactly
  as it does for a skipped blank/non-directory answer.
- Given neither `<name>` nor `--all` is given (or both are given), when
  `lore update` runs, then it fails with a clear error rather than
  guessing intent (the missing-both message reads `specify a
  skill/behavior/agent name or use --all`).
- In every case above, the source directory's (or agent file's) content is
  never read, written, or deleted — only the symlink and, for behaviors, the
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
