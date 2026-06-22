# Architecture & design choices

## Overview

```
┌─────────────┐     Unix socket      ┌──────────────────────────────┐
│  ltw tui    │◄──── JSON lines ────►│         ltw daemon           │
│  ltw add…   │                      │  poller │ server │ SQLite   │
└─────────────┘                      └──────────┬───────────────────┘
                                                │
                                    Yahoo ──────┤ HTTP
                                    brapi ──────┘
```

One Rust crate, one binary (`ltw`), module boundaries instead of a multi-crate workspace. The layout can graduate to a Cargo workspace later if the project grows.

## Why a daemon?

**Problem:** only one process should own the database and the market-data poller. If the TUI and CLI each opened the DB and polled independently, you would get lock contention, duplicate HTTP traffic, and inconsistent snapshots.

**Solution:** a long-running `ltw daemon` is the sole writer. TUI and CLI are thin IPC clients. The CLI auto-spawns the daemon when the socket is missing so interactive use stays frictionless.

## Why SQLite instead of DuckDB?

The original design ([design spec](superpowers/specs/2026-06-21-local-ticker-wallet-design.md)) targeted DuckDB. The implementation uses **SQLite** via `rusqlite` with bundled SQLite:

- Single-writer model matches the daemon architecture naturally.
- No cross-process DuckDB lock issues to work around.
- Embedded, zero-config, excellent for a personal wallet dataset.
- Decimal values are stored as text columns and parsed with `rust_decimal` at the boundary.

The schema preserves the same tables and composite `(symbol, exchange)` keys so multi-market support remains straightforward.

## Module map

| Module | Responsibility | I/O |
|--------|----------------|-----|
| `core` | Domain types, P&L, signals, opportunity score — **pure**, unit-tested | None |
| `storage` | Schema, migrations, queries — **daemon only** | SQLite |
| `providers` | `Provider` trait; Yahoo + brapi; fallback chain | HTTP |
| `ipc` | Request/response envelope, socket framing | Socket |
| `daemon` | Poller, recompute, Unix socket server | — |
| `tui` | ratatui screens, overlays, IPC client | Socket |
| `client` | CLI IPC client, daemon auto-spawn | Socket |
| `portfolio` | CSV import/export | Files |
| `i18n` | `pt-BR` and `en` string bundles | — |

## Data model

The **ledger** (`transactions`) is the source of truth. Everything else is cache or metadata:

- `quotes` — latest price snapshot per asset
- `price_history` — OHLCV for SMAs, 52w range, window returns
- `dividends` — proventos for yield metrics
- `position_snapshots` — materialized position + score; always reconstructible from transactions
- `search_cache` — TTL'd search results; never merged into the portfolio

Migrations are versioned with a SHA-256 checksum of the SQL. A checksum mismatch refuses to start, preventing silent schema drift.

## IPC protocol

Unix domain socket, **newline-delimited JSON** (one object per line), `serde` + `tokio::BufReader::lines`.

Request:

```json
{ "id": "uuid", "type": "request", "action": "AddTransaction", "payload": { } }
```

Success:

```json
{ "id": "uuid", "status": "ok", "data": { } }
```

Error:

```json
{ "id": "uuid", "status": "error", "error": { "code": "NOT_FOUND", "message": "..." } }
```

The `id` is echoed so clients can match responses on a multiplexed socket. Error codes: `NOT_FOUND`, `PROVIDER_DOWN`, `BAD_REQUEST`, `INTERNAL`.

## Polling & trading hours

The poller gates on B3 session hours (weekdays, ~10:00–18:00 BRT). During the session it fetches quotes every `poll_interval_secs` (default 60). Off-hours it sleeps; history and dividends refresh once per day. A full B3 holiday calendar is deferred — weekends and the hours window are sufficient for MVP.

`RefreshNow` (CLI `ltw refresh` or TUI action) bypasses the schedule.

## Provider fallback

```text
Yahoo (primary) ──error/timeout/empty──► brapi (fallback)
```

Each successful quote records `source` in the database. The chain uses backoff and minimum request intervals to respect rate limits.

## Opportunity score (core)

All scoring lives in `core::score` as pure functions over `Quote`, `Candle`, `Dividend`, and `Position` data. Weights come from config. The TUI renders `score_breakdown` JSON stored in `position_snapshots`.

The **cost basis vs trend** sub-score is deliberate: being underwater is only attractive when the trend is turning up (SMA50 slope), reducing "falling knife" false positives.

## Error handling

| Layer | Behavior |
|-------|----------|
| Providers | Log and continue; keep stale cache |
| IPC | Structured error response; connection per request on server |
| DB | Checksum mismatch → refuse start; transactional writes |
| Validation | Reject negative quantity, oversell, unknown tickers in `core` |

Stale sockets (daemon crashed) are removed on the next client connect before auto-spawn.

## Testing strategy

| Layer | Approach |
|-------|----------|
| `core` | Pure unit tests (P&L, signals, score) |
| `providers` | wiremock + fixture JSON |
| `ipc` / daemon | `tests/daemon_ipc.rs` integration tests |
| `e2e` | `tests/e2e.rs` — smoke test; **excluded from CI** (stdout pipe deadlock with auto-spawn) |

Run the CI-equivalent suite locally:

```bash
cargo nextest run --lib --test daemon_ipc
```

## Release & distribution

Releases are automated with [cargo-dist](https://github.com/axodotdev/cargo-dist): tagged semver pushes build Linux/macOS archives, shell installers, and Homebrew formulas. Configuration lives in `Cargo.toml` under `[workspace.metadata.dist]`.
