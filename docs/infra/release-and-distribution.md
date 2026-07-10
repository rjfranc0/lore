# Release & Distribution

## What this covers

How a commit becomes a tagged release with downloadable binaries, and how
`install.sh` decides what to actually put on a user's machine. Application
behavior lives in `implementation/`; this file is purely the path from
source to an installed binary.

## CI gate

Every push (any branch) and every PR runs `cargo test` then `cargo clippy
-- -D warnings` (`.github/workflows/ci.yml`). This is the only required
gate — `cargo fmt --check` is **not** run in CI and is not enforced
anywhere; the repo's actual formatting already drifts from rustfmt
defaults, and there is a standing decision not to force a repo-wide
reformat just to satisfy a check nothing currently requires.

> 🔧 **Manual step**: contributors can opt into a local pre-push hook
> (`.githooks/pre-push`, activated via `git config core.hooksPath
> .githooks`) that runs the same `cargo test` + `cargo clippy -D warnings`
> before a push leaves the machine. This is opt-in, not forced — cloning
> the repo alone does not enable it.

## Branch model: `main` (stable) vs. `dev` (beta)

Two long-lived branches, each watched independently by `release-please`
(`.github/workflows/release-please.yml`):
- **`main`** — stable channel. `release-please-action` runs in its default
  mode.
- **`dev`** — beta/pre-release channel. Same action, with `prerelease:
  true, prerelease-type: beta`.

`release-please` parses the **squash-merge commit subject** of every merge
into either branch — not the individual commits inside the PR — looking
for a Conventional Commit type (`feat:`, `fix:`, `chore:`, etc.) per
`release-please-config.json`'s `changelog-sections`. A PR merged with a
non-conventional squash title produces no release-please activity at all,
silently.
> ⚠️ **Inferred**: "silently" here is read from how release-please's
> documented behavior works, not from a test in this repo — there is no
> automated check in this codebase that *catches* a malformed
> squash-commit title before merge.

`include-component-in-tag: false` in the config means tags are plain
`v<version>` (e.g. `v0.2.0`), not `lore-v0.2.0` — this must keep matching
the `tags: ["v*"]` trigger in `release.yml` (below); changing one without
the other breaks the release pipeline silently (the tag would be pushed,
but no build would trigger).

To pin an exact version on a release PR (e.g. bootstrapping a first
release, or correcting drift), a `Release-As: X.Y.Z` footer on the
triggering commit overrides release-please's own version inference.

## Release build matrix

Triggered by a pushed `v*` tag, or manually via `workflow_dispatch` with a
`tag` input (`.github/workflows/release.yml`) — never by merging to
`main`/`dev` directly; the tag is what release-please's own PR-merge
ultimately produces. The manual path exists to recover a release that was
created without a build (see the `RELEASE_PLEASE_TOKEN` note below); both
paths resolve to the same `env.RELEASE_TAG`, which drives checkout ref,
`tag_name`, and the `prerelease` check. Builds 4 targets in parallel:

| Target | Runner | Cross-compiled via `cross`? |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | no |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` | yes |
| `x86_64-apple-darwin` | `macos-13` | no |
| `aarch64-apple-darwin` | `macos-latest` | no |

Each artifact is named `lore-<os>-<arch>` and uploaded to the GitHub
Release matching the pushed tag. `prerelease: ${{ contains(github.ref_name,
'-') }}` — a tag is marked prerelease on GitHub purely by whether its
version string contains a hyphen (e.g. `v0.3.0-beta.1`), which is exactly
the shape `release-please`'s beta channel produces on `dev`.

## `install.sh` — channel and source selection

Reads one optional argument, `--dev` (anything else is a hard error). Picks
platform via `uname -s`/`uname -m`, mapped to the same `lore-<os>-<arch>`
artifact-naming scheme the release workflow produces — **these two naming
schemes must stay in sync by hand**; there is no shared source between the
bash script and the YAML workflow.

**Dev channel** (`--dev`): queries the GitHub releases API directly, greps
for the first `browser_download_url` matching the platform's artifact
name, downloads it. No local-build path — dev-channel installs are always
a downloaded pre-release binary.

**Stable channel** (default), in priority order:
1. If running from a local clone that already has `target/release/lore`
   built — copy it directly (fastest path, used right after a local
   `cargo build --release`).
2. Else, if running from a local clone with `Cargo.toml` present and
   `cargo` available — build from source (`cargo build --release
   --quiet`), then copy.
3. Else — download the latest stable release artifact from GitHub.

This ordering means **running `./install.sh` from a freshly-cloned repo
always prefers building from source** over downloading, even on the stable
channel — only the `curl | bash` one-liner (no local clone) ever hits the
download path on stable.

After installing, `check_path` warns (does not fail) if `$INSTALL_DIR`
(default `~/.local/bin`) isn't on `$PATH`, and prints the exact shell
config line to add.

## What breaks if this is touched

- Changing `include-component-in-tag` without updating `release.yml`'s
  `tags: ["v*"]` trigger breaks the release pipeline — release-please
  would push a tag in a shape the build workflow never fires on.
- Changing the release artifact naming (`lore-<os>-<arch>`) in
  `release.yml` without updating `detect_platform()` in `install.sh`
  breaks `install.sh` for every channel that downloads rather than builds
  locally.
- Merging to `main`/`dev` with a non-conventional squash-commit subject
  produces no version bump and no changelog entry for that change —
  silently, with no CI failure to catch it. **Confirmed**: the same space
  before the colon (`feat : ...` instead of `feat: ...`) breaks the
  conventional-commit parser too — PRs #5 and #7 both used that shape and
  produced no release-please activity at all.
- `release-please.yml`'s `googleapis/release-please-action` step must
  authenticate with a PAT (`secrets.RELEASE_PLEASE_TOKEN`), not
  `secrets.GITHUB_TOKEN`. **Confirmed**: GitHub suppresses downstream
  workflow triggers (including both `push: tags` and `release: published`)
  for any tag/release created using the default `GITHUB_TOKEN`, to prevent
  recursive workflow runs. With `GITHUB_TOKEN`, release-please still tags
  and publishes the GitHub Release, but `release.yml` never fires — the
  release silently ships with zero binaries attached. This is exactly what
  happened to `v0.2.0` and initially to `v1.0.0` before the token was
  switched to a PAT; `release.yml`'s `workflow_dispatch` input exists
  specifically to rebuild/attach binaries to a tag that shipped this way.
