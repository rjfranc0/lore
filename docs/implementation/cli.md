# CLI

## Purpose

Owns argument parsing (`cli.rs`) and the dispatch from a parsed command to
its implementation (`lib.rs`'s `run()`). This is the only layer that knows
about `clap` — every `commands::*` function takes plain,
already-validated-by-clap argument types and knows nothing about how it was
invoked.

## Module: `cli.rs`

**Responsibility**: defines the entire command surface as clap derive
types. Owns nothing at runtime — it's pure schema.

```rust
pub struct Cli { pub command: Option<Command> }

pub enum Command {
    Init { account: Option<String> },
    Install { skills: Vec<String> },      // #[arg(required = true)]
    Remove  { skills: Vec<String> },      // #[arg(required = true)]
    Behavior { action: BehaviorAction },
    Accounts { action: AccountsAction },
    Sync,
    Update { name: Option<String>, all: bool, path: Option<String> },
    List,
    Version,
    Help,
}

pub enum BehaviorAction { Add { names: Vec<String> }, Remove { names: Vec<String> } }
pub enum AccountsAction { List, Remove { name: String }, Sync }
```

`Cli` is built with `disable_version_flag = true` and
`disable_help_subcommand = true` — both `version`/`help` are handled as
ordinary `Command` variants instead of clap's built-ins, so their behavior
(exit codes, output formatting) is under this codebase's control rather
than clap's defaults. See "Help text duplication" below for the cost of
that choice.

## Module: `lib.rs` — dispatch

**Responsibility**: turns a parsed `Cli` into an `ExitCode`. This is the
only place that maps a `Command` variant to a `commands::` function — the
mapping is 1:1 and exhaustive (every `Command` variant has exactly one
arm).

**Parse-failure contract**: `Cli::try_parse()` failing is split into two
cases by `clap::error::ErrorKind`:
- `DisplayHelp` (`--help`/`-h` — clap still intercepts this as a flag even
  though the `help` *subcommand* is disabled above) — print clap's own
  message, exit 0.
- Anything else (unknown subcommand, bad arguments) — print `SHORT_HELP` to
  **stderr**, exit 1.

**`--version`/`-V` are not a parallel case to `--help`.**
`disable_version_flag = true` removes clap's auto-generated version flag
entirely, so both fall straight into the "anything else" branch above —
exit 1, `SHORT_HELP`, identical to an unrecognized argument. Confirmed by
running the built binary: `lore --version` and `lore -V` both exit 1. The
`DisplayVersion` arm still present in `lib.rs`'s `match` is dead code under
this configuration — `lore version` (the subcommand, an ordinary
`Command::Version` dispatch, not a parse failure) is the only way to print
the version string today.

This is why an unknown subcommand and bare `lore` (no args) look almost
identical on screen but differ in exit code and stream: bare `lore` prints
`SHORT_HELP` to **stdout** and exits 0 (a no-op query, not an error); an
unknown subcommand prints the same text to **stderr** and exits 1 (so a
typo in a script is detectable, not silently "successful").

**Error contract for every dispatched command**: every `commands::*`
function returns `anyhow::Result<()>`. `lib.rs::run()` is the single place
that turns `Err(e)` into user-visible output —
`eprintln!("✗ {e}")` plus `ExitCode::FAILURE`. No command module prints its
own top-level failure message; they only `bail!`/`?` upward, see
[@/implementation/agent-config.md] and [@/implementation/accounts.md] for
what each command actually does once dispatched.

## Help text duplication (known sharp edge)

Two independent strings describe lore's command surface, and neither is
generated from the other:
- `SHORT_HELP` (a `const` in `lib.rs`) — printed for bare `lore`, an
  unknown subcommand, and as part of every hard failure from
  `Cli::try_parse()`.
- `help.txt` (embedded via `include_str!` in `commands/help.rs`, printed by
  `lore help`) — the full manual, piped through `$PAGER` (default `less`)
  if one spawns successfully, otherwise printed directly to stdout.

> ⚠️ **Inferred:** there is no test or build step that checks these two
> stay consistent with each other, or with `cli.rs`'s actual `Command`
> enum. Adding a new subcommand requires remembering to update
> `SHORT_HELP`, `help.txt`, *and* `cli.rs` by hand — confirmed by reading
> all three, not stated anywhere as a rule. A change to one without the
> other two is a silent doc-drift bug, not a compile error.

**Confirmed instance**: the `update` subcommand updated `SHORT_HELP` (see
[@/functional/agent-config.md#feature-update]) but not `help.txt` —
`lore help` does not mention `update` anywhere (COMMANDS, FILES, or
EXAMPLES) as of this writing. This is a product gap, not just a doc one;
`docs/` cannot fix it, since `help.txt` is the source of truth it would be
documenting.

## What breaks if this is touched

- Adding a `Command` variant without a matching arm in `lib.rs::run()`'s
  `match` is a compile error (exhaustive match) — this part is safe by
  construction.
- Adding a `Command` variant without updating `SHORT_HELP`/`help.txt`
  compiles fine and silently ships an undocumented command — see above.
- Changing the `DisplayHelp` vs. "everything else" split changes which
  failures exit 0 vs. 1 — any script depending on lore's exit codes is
  sensitive to this. The `DisplayVersion` arm is currently dead code (see
  above) — it has nothing to change yet, but re-enabling the version flag
  in the future would make this same sensitivity apply to it too.
