# Module: `commands/accounts.rs`

**`list`** — pure read of `config.accounts` (already sorted, `BTreeMap`),
no disk check.

**`remove`** — pure mutation of `config.accounts`, then `save`. Warns (does
not error) for an unregistered name or for `"default"` specifically, but
performs the removal either way for `"default"` (the warning is
informational, not a refusal).

**`sync`** — for each registered account, the wiring check verifies *two*
hops for CLAUDE.md/LORE.md, plus a skills check that flipped polarity with
this feature's real-directory model:
```rust
let already_wired = claude_md.exists()
    && std::fs::read_to_string(&claude_md)
        .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", lore_md.display())))
    && lore_md.exists()
    && std::fs::read_to_string(&lore_md)
        .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", agents_md.display())))
    && claude_skills.is_dir()
    && !symlink::is_link(&claude_skills);
```
**The skills check used to require a *live symlink*; it now requires the
opposite — a real directory that is *not* a symlink.** This mirrors
`wire_claude_skills`'s shape change (see [wire](wire.md)): a symlink
sitting at `claude_dir/skills` is now the **broken** state (the legacy
single-symlink model), not the correct one. This check is shape-only — it
confirms the directory exists and isn't a stray symlink, not that every
shared skill's re-link is actually present inside it; deep re-link
reconciliation is separate, tracked future work.

**A read failure on either CLAUDE.md or LORE.md (permission denied,
non-UTF-8 content) folds into `false` via `is_ok_and`** — treated
identically to "wrong content," not surfaced as a distinct error. This is
deliberate: `sync`'s whole purpose is self-healing, so routing every form
of "not correct" through the same `wire_claude_dir` rewrite path (rather
than carving out a separate branch for unreadable-but-possibly-fixable
files) keeps the function's logic to one path instead of two. If
`wire_claude_dir` itself then fails (e.g. genuine permission denial on
write), that error still propagates normally — only the *read* used for
the "is it already correct" check is swallowed, not the *write* used to
fix it.

If not already wired, `sync` computes the same migration-target tuple
[init](init.md) does (keyed on `name == "default"`) before calling
`wire_claude_dir` — a rewire triggered by `sync` goes through the exact
same surgical CLAUDE.md handling `init` does, never a separate code path.
