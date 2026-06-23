# local-ticker-wallet

[![CI](https://github.com/carvalhosauro/local-ticker-wallet/actions/workflows/ci.yml/badge.svg)](https://github.com/carvalhosauro/local-ticker-wallet/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ltw.svg)](https://crates.io/crates/ltw)
[![GitHub release](https://img.shields.io/github/v/release/carvalhosauro/local-ticker-wallet?include_prereleases&display_name=tag)](https://github.com/carvalhosauro/local-ticker-wallet/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A terminal-first personal stock wallet for tracking B3 (Brazilian) equities. Record your trades locally, poll market data in the background, and review positions, P&L, and opportunity scores in a fast TUI — or via a thin CLI.

**Binary name:** `ltw`

## Install

Pick one method — all install the `ltw` command.

### Debian / Ubuntu (recommended on Linux)

```bash
curl -fsSL https://carvalhosauro.github.io/local-ticker-wallet/install-apt.sh | sudo sh
```

That adds the APT repository and runs `apt install ltw`. Afterwards:

```bash
sudo apt update && sudo apt install ltw
```

### One-liner (Linux & macOS, no root)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/carvalhosauro/local-ticker-wallet/releases/latest/download/ltw-installer.sh | sh
```

Installs to `~/.cargo/bin` (same as Rust's cargo). Restart your shell or run `source ~/.cargo/env`.

### Homebrew (macOS & Linux)

```bash
brew install carvalhosauro/tap/ltw
```

### Rust (crates.io)

```bash
cargo install ltw
```

Requires a Rust toolchain (≥ 1.85). Puts `ltw` in `~/.cargo/bin`.

More options and configuration: [docs/installation.md](docs/installation.md).

## Quick start

```bash
# Interactive use — daemon auto-starts when needed
ltw add PETR4 100 28.50 --date 2026-01-02 --note "first buy"
ltw list
ltw tui
```

For scripts, start the daemon explicitly (see [installation](docs/installation.md#running-in-scripts)).

## Highlights

- **Local-first** — your ledger lives in SQLite on disk; no account or cloud sync required.
- **Daemon + clients** — one long-running process owns the database; the TUI and CLI talk to it over a Unix socket.
- **Transparent scoring** — a configurable 0–100 opportunity score with a per-asset breakdown (not a black box).
- **Resilient market data** — Yahoo Finance (primary) with [brapi.dev](https://brapi.dev) fallback.
- **Portable** — export and import the ledger as CSV to move between machines.

## Documentation

| Topic | Guide |
|-------|-------|
| Installation & configuration | [docs/installation.md](docs/installation.md) |
| Features & CLI reference | [docs/features.md](docs/features.md) |
| Architecture & design choices | [docs/architecture.md](docs/architecture.md) |
| Releases & versioning | [docs/releasing.md](docs/releasing.md) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Security | [SECURITY.md](SECURITY.md) |

## Project status

MVP scope is **B3 stocks** priced in BRL. FIIs, ETFs, US equities, crypto, alerts, and a full B3 holiday calendar are on the roadmap. The schema is multi-market from day one so later asset classes do not require a rewrite.

## Development

```bash
cargo build
cargo clippy --all-targets
cargo nextest run --lib --test daemon_ipc
```

## License

MIT — see [LICENSE](LICENSE).
