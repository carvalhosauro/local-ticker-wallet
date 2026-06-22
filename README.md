# local-ticker-wallet

[![CI](https://github.com/carvalhosauro/local-ticker-wallet/actions/workflows/ci.yml/badge.svg)](https://github.com/carvalhosauro/local-ticker-wallet/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A terminal-first personal stock wallet for tracking B3 (Brazilian) equities. Record your trades locally, poll market data in the background, and review positions, P&L, and opportunity scores in a fast TUI — or via a thin CLI.

**Binary name:** `ltw`

## Highlights

- **Local-first** — your ledger lives in SQLite on disk; no account or cloud sync required.
- **Daemon + clients** — one long-running process owns the database; the TUI and CLI talk to it over a Unix socket.
- **Transparent scoring** — a configurable 0–100 opportunity score with a per-asset breakdown (not a black box).
- **Resilient market data** — Yahoo Finance (primary) with [brapi.dev](https://brapi.dev) fallback.
- **Portable** — export and import the ledger as CSV to move between machines.

## Quick start

### Install from source

```bash
git clone https://github.com/carvalhosauro/local-ticker-wallet.git
cd local-ticker-wallet
cargo install --path .
```

Pre-built binaries for Linux and macOS are published on [GitHub Releases](https://github.com/carvalhosauro/local-ticker-wallet/releases). See [Installation](docs/installation.md) for shell installer, Homebrew, and build requirements.

### First run

```bash
# Start the daemon (redirect output when running in scripts)
ltw daemon >/tmp/ltw-daemon.log 2>&1 &

# Record a buy
ltw add PETR4 100 28.50 --date 2026-01-02 --note "first buy"

# List positions (JSON)
ltw list

# Open the TUI
ltw tui
```

The CLI auto-starts the daemon when the socket is absent. For scripted use, start the daemon yourself and redirect its stdout/stderr — see [Installation](docs/installation.md#running-in-scripts).

## Documentation

| Topic | Guide |
|-------|-------|
| Installation & configuration | [docs/installation.md](docs/installation.md) |
| Features & CLI reference | [docs/features.md](docs/features.md) |
| Architecture & design choices | [docs/architecture.md](docs/architecture.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## Project status

MVP scope is **B3 stocks** priced in BRL. FIIs, ETFs, US equities, crypto, alerts, and a full B3 holiday calendar are on the roadmap. The schema is multi-market from day one so later asset classes do not require a rewrite.

## Development

```bash
cargo build          # dev build
cargo clippy --all-targets
cargo nextest run --lib --test daemon_ipc   # excludes hanging e2e test
```

Rust ≥ 1.85 is required (transitive dependencies use Rust 2024 edition features). The `mold` linker is configured for Linux x86_64 builds — see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
