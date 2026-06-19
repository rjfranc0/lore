# Infra

How a commit becomes a released, installable binary. Separate from
`implementation/` because this is an operational/deployment concern, not
application behavior — touching this affects how lore ships, not what it
does once installed.

- [release-and-distribution](release-and-distribution.md) — CI gate, the
  `main`/`dev` branch + release-please model, the multi-platform release
  build matrix, and `install.sh`'s channel/source-selection logic.
