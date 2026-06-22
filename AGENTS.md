# AGENTS.md

## Cursor Cloud specific instructions

`local-ticker-wallet` is a single-crate Rust app producing one binary, `ltw`, that
runs in three modes:

- `ltw daemon` — long-running process; the **only** owner of the SQLite DB, serves
  a Unix domain socket, and runs the market-data poller.
- `ltw add | list | delete | refresh | search | import | export` — thin CLI clients
  that talk to the daemon over the socket (auto-spawning it if absent).
- `ltw tui` — ratatui terminal UI client (also talks over the socket).

Standard commands (build/lint/test) are plain Cargo; the `cargo t` alias is defined
in `.cargo/config.toml` as `cargo nextest run`. Use:

- Build (dev): `cargo build`
- Lint: `cargo clippy --all-targets`
- Test: `cargo nextest run` (or the `cargo t` alias)

### Toolchain / build caveats

- **Rust ≥ 1.85 is required** even though `Cargo.toml` says `rust-version = "1.75"`:
  the committed `Cargo.lock` pins dependencies (e.g. `hashbrown 0.17`) that need
  `edition2024`. The snapshot ships current stable (1.96+) as the default toolchain.
- **The `mold` linker is mandatory** — `.cargo/config.toml` sets
  `-fuse-ld=mold` for the `x86_64-unknown-linux-gnu` target, so a build fails
  without it. It is preinstalled in the snapshot (`apt` package `mold`).
- `cargo-nextest` is installed from its prebuilt binary (the crates.io version
  needs a newer rustc than is sometimes available); `cargo nextest run` works.

### Testing caveats

- `cargo fmt --check` reports a diff in `src/main.rs` (match-arm wrapping). This is
  only rustfmt-version drift between the installed `rustfmt` and the one the repo
  was formatted with — it is **not** a code error. `cargo clippy` is clean.
- **`tests/e2e.rs` hangs and must be excluded from automated runs.** Its helper uses
  `std::process::Command::output()`, which reads the child's stdout to EOF — but the
  CLI auto-spawns a background `ltw daemon` that **inherits that captured stdout pipe
  and never closes it**, so `output()` blocks forever. This is a code/test design
  issue, not an environment problem; the underlying flow works when run normally.
  Run the rest of the suite with:
  `cargo nextest run --lib --test daemon_ipc` (37 tests pass).

### Running the app (gotcha)

Because the CLI auto-spawns the daemon as a child that **inherits the caller's
stdout/stderr**, running a CLI command whose stdout is a *pipe* (captured by
`$(...)`, `| tee`, `Command::output()`, etc.) will block until EOF that never comes.
To run reliably, start the daemon yourself with its output redirected, then use the
CLI:

```
ltw daemon >/tmp/ltw-daemon.log 2>&1 &
ltw add PETR4 100 28.50 --date 2026-01-02 --note "first buy"
ltw list
```

Interactive terminal use (plain `ltw tui`, or CLI to a TTY) is unaffected.

### Data locations & network

- DB: `<XDG_DATA_HOME>/local-ticker-wallet/wallet.db` (SQLite, despite the design doc
  mentioning DuckDB the implementation uses `rusqlite`).
- Config: `<XDG_CONFIG_HOME>/local-ticker-wallet/config.json`.
- Socket: `$XDG_RUNTIME_DIR/local-ticker-wallet.sock` (falls back to a temp dir).
- Override the `XDG_*` vars to isolate a throwaway wallet (the tests do this).
- `refresh`/`search` and the daemon poller fetch from Yahoo (`query1.finance.yahoo.com`)
  with `brapi.dev` fallback and need internet; `add`/`list`/`delete`/`export`/`import`
  work fully offline.
