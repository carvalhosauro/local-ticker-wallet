# Releasing

This document describes how versions, git tags, and GitHub Releases work in this project.

## Overview

```
CHANGELOG.md  →  cargo release  →  git tag vX.Y.Z  →  GitHub Actions (cargo-dist)
                                              ↓
                         binaries · crates.io · APT · Homebrew
```

| Step | Tool | What happens |
|------|------|--------------|
| 1. Write notes | `CHANGELOG.md` | Edit the `[Unreleased]` section |
| 2. Bump version | [cargo-release](https://github.com/crate-ci/cargo-release) | Updates `Cargo.toml`, commits |
| 3. Tag | `cargo release` | Creates annotated tag `vX.Y.Z` |
| 4. Publish | GitHub Actions | Builds artifacts and publishes everywhere |

Release notes on GitHub are generated from `CHANGELOG.md` by [cargo-dist](https://github.com/axodotdev/cargo-dist).

## Versioning

We follow [Semantic Versioning](https://semver.org/):

| Bump | When |
|------|------|
| **MAJOR** (`1.0.0`) | Breaking CLI/IPC changes, DB migrations that are not backward-compatible |
| **MINOR** (`0.2.0`) | New features, new asset classes, backward-compatible schema changes |
| **PATCH** (`0.1.1`) | Bug fixes, dependency updates, docs-only changes that ship a rebuild |

While in `0.x.y`, treat **MINOR** as the primary feature bump and **PATCH** for fixes.

### Pre-releases

Use semver pre-release identifiers:

| Tag | Meaning |
|-----|---------|
| `v0.2.0-alpha.1` | Early testing |
| `v0.2.0-beta.1` | Feature-complete, stabilizing |
| `v0.2.0-rc.1` | Release candidate |

Pre-releases are marked as such on GitHub. Set `publish-prereleases = true` in `[workspace.metadata.dist]` if you want them on Homebrew/crates.io/APT (default: skipped for most channels).

## Tag format

**Always use an annotated tag with a `v` prefix:**

```
vMAJOR.MINOR.PATCH
```

Examples: `v0.1.0`, `v0.2.0-beta.1`

cargo-dist listens for tags matching `*[0-9]+.[0-9]+.[0-9]+*`. A tag like `v0.1.0` triggers a **unified** release for the `ltw` crate.

Do **not** use bare `0.1.0` (without `v`) unless you intentionally want a different dist parsing mode.

## Changelog

Edit `CHANGELOG.md` before every release:

1. Move items from `[Unreleased]` into a new `## [X.Y.Z] - YYYY-MM-DD` section.
2. Leave empty `### Added/Changed/Fixed/Removed` headings under `[Unreleased]` for the next cycle.
3. Update the comparison links at the bottom of the file.

[cargo-dist reads this file](https://axodotdev.github.io/cargo-dist/book/workspaces/simple-guide.html) and uses the matching version heading as the GitHub Release title and body.

**Keep the `[Unreleased]` heading name** — cargo-release and cargo-dist rewrite it automatically.

## Release checklist (maintainers)

### Prerequisites (one-time)

- [ ] `CARGO_REGISTRY_TOKEN` secret configured
- [ ] `HOMEBREW_TAP_TOKEN` secret configured
- [ ] GitHub Pages enabled (source: **GitHub Actions**) for the APT repository
- [ ] `cargo install cargo-release cargo-dist` locally

### Every release

1. **Prepare the changelog**
   ```bash
   git checkout main && git pull
   # Edit CHANGELOG.md — fill [Unreleased], add [X.Y.Z] section
   git add CHANGELOG.md
   git commit -m "docs(changelog): prepare vX.Y.Z"
   ```

2. **Dry-run** (no side effects)
   ```bash
   cargo release X.Y.Z --dry-run
   dist plan --tag=vX.Y.Z
   ```

3. **Option A — direct release** (solo maintainer, push to `main` allowed)
   ```bash
   cargo release X.Y.Z --execute --no-publish
   ```
   `--no-publish` because crates.io is published by CI. The command will:
   - bump `Cargo.toml` version
   - commit `chore(release): ltw X.Y.Z`
   - create tag `vX.Y.Z`
   - push commit + tag

4. **Option B — release via PR** (recommended for teams)
   ```bash
   git checkout -b release/vX.Y.Z
   cargo release X.Y.Z --no-publish --no-tag --allow-branch=release/vX.Y.Z --execute
   git push -u origin release/vX.Y.Z
   # Open PR, review, squash-merge to main
   # Then on main:
   git checkout main && git pull
   dist plan --tag=vX.Y.Z
   cargo release X.Y.Z --execute --no-publish
   ```

5. **Wait for CI** — the [Release workflow](https://github.com/carvalhosauro/local-ticker-wallet/actions/workflows/release.yml) runs on the tag push and:
   - builds Linux/macOS tarballs + `ltw-installer.sh`
   - creates the GitHub Release with changelog notes
   - publishes to crates.io, Homebrew tap, and APT (GitHub Pages)

6. **Verify**
   - [GitHub Releases](https://github.com/carvalhosauro/local-ticker-wallet/releases)
   - [crates.io/crates/ltw](https://crates.io/crates/ltw)
   - `curl -LsSf …/ltw-installer.sh | sh` on a clean machine
   - `sudo apt update && sudo apt install ltw` (after APT index propagates)

## What each channel publishes

| Channel | Trigger | Artifact |
|---------|---------|----------|
| GitHub Releases | tag push | `.tar.xz`, installer script, checksums |
| crates.io | `publish-crates-io.yml` | `cargo install ltw` |
| Homebrew | `publish-homebrew-formula` job | `brew install carvalhosauro/tap/ltw` |
| APT | `publish-apt.yml` | `sudo apt install ltw` |

## Hotfix (patch) release

```bash
git checkout main && git pull
# Fix the bug on main (or cherry-pick), update CHANGELOG [Unreleased]
cargo release X.Y.Z --execute --no-publish   # e.g. 0.1.1
```

## Rolling back a bad release

1. **Yank** the crates.io version: `cargo yank --vers X.Y.Z ltw`
2. **Mark** the GitHub Release as pre-release or add a warning in the description
3. Ship `X.Y.Z+1` with the fix — tags cannot be reused once pushed

## Local dist commands

```bash
dist plan                  # what would be released at current version
dist plan --tag=v0.2.0     # plan for a specific tag
dist build                 # build artifacts locally (slow, cross-compile)
```

## Related

- [Installation](installation.md) — how users install each channel
- [CONTRIBUTING.md](../CONTRIBUTING.md) — development workflow
- [cargo-dist tag reference](https://axodotdev.github.io/cargo-dist/book/reference/announcement-tags.html)
