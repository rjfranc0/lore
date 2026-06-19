# Manual test report — multi-account Claude support (RJ-54)

Run against `./target/debug/lore`, each scenario in its own fresh
`mktemp -d` sandbox with `HOME`/`LORE_CONF` pointed inside it. Nothing
touched the real machine.

This is a **re-run** of the full plan after fixing the bug the first pass
found (see History below) — every scenario below was re-executed from
scratch against the fixed binary.

## Results

| # | Scenario | Result |
|---|---|---|
| 1 | First-run bootstrap | PASS |
| 2 | Multiple accounts share one agents tree | PASS |
| 3 | `accounts remove` is registry-only, reversible | PASS |
| 4 | `accounts sync` heals broken `CLAUDE.md` / symlink / real-directory `skills` path | PASS |
| 5 | `LORE_CONF` override isolates state | PASS |
| 6 | Skill-migration collision hard-stop + resolve-and-retry | PASS |

All 6 scenarios pass. No outstanding bugs from this test plan.

## Scenario 4 detail (previously failing, now fixed)

Repro: in a sandbox with an initialized `work` account, delete `CLAUDE.md`
and replace the healthy `skills` symlink with a plain directory:
```bash
rm ~/.claude-work/CLAUDE.md
rm ~/.claude-work/skills
mkdir ~/.claude-work/skills
lore accounts sync
```
Before the fix this exited 1 with `File exists (os error 17)` —
`src/wire.rs`'s skills relink only removed the existing `claude_skills`
path when it was already a symlink, so a real directory at that path was
left in place and `symlink::create()` failed with EEXIST.

Re-run against the fixed binary:
```
✓ Wrote /…/.claude-work/CLAUDE.md
✓ Linked /…/.claude-work/skills → /…/.agents/skills
✓ Re-wired account: work → /…/.claude-work
exit=0
```
A second `accounts sync` immediately after correctly reports
`✓ Accounts already in sync` (idempotent, no spurious rewrite).

## History

- **First pass** (this same plan, prior binary): scenario 4 failed with
  the EEXIST crash above. Root cause and fix detailed in the dispatch
  comment to RJ-54 (Linear comment `403d3d19`) — `src/wire.rs`'s skills
  relink now removes a real directory (`remove_dir_all`) or plain file
  (`remove_file`) occupying `claude_skills`, not just a symlink. Added
  regression test `tests/integration/accounts.rs::sync_rewires_skills_path_replaced_by_real_directory`.
  `cargo clippy --all-targets --all-features -- -D warnings` clean,
  `cargo test` 63/63 green.
- **This pass**: full re-run of all 6 scenarios from scratch against the
  rebuilt binary, confirming the fix and re-confirming the other 5
  scenarios still pass unchanged.

## Notes

- Scenario 1/2/5: a stray `[rtk] /!\ No hook installed...` line appears
  occasionally in `cat` output — that's the `rtk` shell wrapper around
  `cat`, unrelated to `lore`; not a real finding.
- Scenario 6 requires using `~/.claude/skills` (the actual migration
  path, `wire::claude_skills_path` = `claude_dir.join("skills")`), not a
  `claude_skills`-named directory. Behavior matches
  `docs/implementation/accounts.md` exactly: collision left in place and
  warned, movable skill migrated, bail before `CLAUDE.md` is wired, exit 1,
  account left unregistered; resolving the collision and re-running heals
  cleanly to exit 0.
