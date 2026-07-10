# Accounts — Implementation

How the account/config layer implements the behavior specified in
[@/functional/accounts.md]. Each module below owns a distinct piece of the
`lore.toml` → Claude-account wiring chain; read [config](config.md) first
if you're new to this layer, since every other module depends on it.

## Modules

- [config](config.md) — `config.rs` (`LoreConfig`, `lore.toml`) and
  `paths.rs` (derived agents-dir paths). The base layer every other module
  in this domain builds on.
- [wire](wire.md) — `wire.rs`: the only module that knows a Claude
  account directory's on-disk layout and actually writes one.
- [init](init.md) — `commands/init.rs`: resolves which Claude directory an
  invocation targets, runs migration, triggers registration.
- [commands](commands.md) — `commands/accounts.rs`: `accounts list` /
  `remove` / `sync`.
