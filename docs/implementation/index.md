# Implementation

How lore's code is structured — module responsibilities, contracts, and the
architectural decisions behind the non-obvious parts. Each file here
implements the behavior specified in the matching
[functional](../functional/index.md) file; read that first for the *what*,
this for the *how*.

- [cli](cli.md) — argument parsing (`cli.rs`) and dispatch (`lib.rs`) — the
  layer underneath every other module here.
- [agent-config](agent-config.md) — `agents_md.rs`, `symlink.rs`,
  `output.rs`, and the skill/behavior/sync/list/update commands built on
  them.
- [accounts](accounts.md) — `config.rs`, `paths.rs`, `wire.rs`, and the
  `init`/`accounts` commands built on them.
