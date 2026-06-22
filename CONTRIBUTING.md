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
3. Ensure `cargo clippy --all-targets` and `cargo nextest run --lib --test daemon_ipc` pass.
4. Open a pull request describing what changed and why.

CI runs on every push and pull request — see `.github/workflows/ci.yml`.

## Code organization

Read [docs/architecture.md](docs/architecture.md) before large changes. Guiding principles:

- **`core` stays pure** — no I/O, heavy unit test coverage.
- **Only the daemon touches SQLite** — clients go through IPC.
- **Minimize dependencies** — prefer std and existing crates.

## Reporting bugs

Include OS, Rust version (`rustc --version`), how you invoked `ltw`, and relevant log output. For daemon issues, run `ltw daemon >/tmp/ltw-daemon.log 2>&1` and attach the log.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
