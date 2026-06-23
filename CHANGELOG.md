# Changelog

All notable changes to **ltw** (local-ticker-wallet) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Fixed

### Removed

## [0.1.0] - 2026-06-22

### Added

- Single-binary wallet (`ltw`) with daemon, CLI, and ratatui TUI
- B3 stock ledger: buy/sell transactions, P&L, opportunity score (0–100)
- Background market-data poller (Yahoo primary, brapi.dev fallback)
- Portfolio screens: list, detail with braille chart, ledger, search
- CSV export/import for portable ledger backup
- i18n: `pt-BR` (default) and `en`
- Distribution: shell installer, Homebrew tap, APT repo, crates.io (`cargo install ltw`)
- SQLite storage with versioned migrations

[Unreleased]: https://github.com/carvalhosauro/local-ticker-wallet/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/carvalhosauro/local-ticker-wallet/releases/tag/v0.1.0
