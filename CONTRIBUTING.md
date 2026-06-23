# Contributing

Thanks for your interest in local-ticker-wallet! This project is early-stage; issues and pull requests are welcome.

## Getting started

```bash
git clone https://github.com/carvalhosauro/local-ticker-wallet.git
cd local-ticker-wallet
cargo build
```

### Toolchain

- **Rust ≥ 1.85** — required by transitive dependencies (`edition2024` features in the lockfile).
- **mold linker** — mandatory on Linux x86_64 (see `.cargo/config.toml`). Install with `apt install mold` on Debian/Ubuntu.
- **cargo-nextest** — used in CI; install from [nextest.rs](https://nexte.st/) or use `cargo test` locally.

### Running tests

```bash
cargo nextest run --lib --test daemon_ipc
```

Do **not** rely on `tests/e2e.rs` in automated runs: it uses `Command::output()`, which deadlocks when the CLI auto-spawns a daemon that inherits the captured stdout pipe.

### Linting

```bash
cargo clippy --all-targets -- -D warnings
```

`cargo fmt --check` may report diffs due to rustfmt version drift between environments; clippy is the authoritative style gate in CI.

## Pull request workflow

1. Fork and create a branch from `main`.
2. Make focused changes with clear commit messages.
3. Update `CHANGELOG.md` under `[Unreleased]` for user-facing changes.
4. Ensure `cargo clippy --all-targets` and `cargo nextest run --lib --test daemon_ipc` pass.
5. Open a pull request — the template includes a changelog checklist.

CI runs on every push and pull request — see `.github/workflows/ci.yml`.

PR labels (`feature`, `fix`, `documentation`, …) are used by [Release Drafter](https://github.com/release-drafter/release-drafter) to group changes in the draft GitHub Release after merge to `main`.

## Code organization

Read [docs/architecture.md](docs/architecture.md) before large changes. Guiding principles:

- **`core` stays pure** — no I/O, heavy unit test coverage.
- **Only the daemon touches SQLite** — clients go through IPC.
- **Minimize dependencies** — prefer std and existing crates.

## Reporting bugs

Include OS, Rust version (`rustc --version`), how you invoked `ltw`, and relevant log output. For daemon issues, run `ltw daemon >/tmp/ltw-daemon.log 2>&1` and attach the log.

## Releasing (maintainers)

Full guide: **[docs/releasing.md](docs/releasing.md)**

Quick summary:

1. Edit `CHANGELOG.md` (`[Unreleased]` → new `[X.Y.Z]` section)
2. `cargo release X.Y.Z --execute --no-publish` (bumps version, tags `vX.Y.Z`, pushes)
3. CI builds binaries and publishes to GitHub Releases, crates.io, Homebrew, APT

Required repository secrets:

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | Publish `ltw` to [crates.io](https://crates.io/crates/ltw) |
| `HOMEBREW_TAP_TOKEN` | Push Homebrew formula to `carvalhosauro/homebrew-tap` |

Also enable **GitHub Pages** (source: GitHub Actions) for the APT repository.

The Release Drafter workflow maintains a **draft** GitHub Release with merged PR summaries — use it as a reference when writing `CHANGELOG.md`. The official release is created by cargo-dist when the git tag is pushed.

Dry-run locally:

```bash
cargo release X.Y.Z --dry-run
cargo dist plan --tag=vX.Y.Z
```

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
