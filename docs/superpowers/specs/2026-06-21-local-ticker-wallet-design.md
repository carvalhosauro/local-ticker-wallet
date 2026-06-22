# local-ticker-wallet — Design

**Date:** 2026-06-21
**Status:** Approved (design phase)

## Overview

A terminal (TUI) personal stock wallet for tracking the user's own assets. A
background daemon polls market data, maintains a local DuckDB cache, and computes
position metrics; a thin TUI client renders them. The tool is intended to be
open-source and developer-focused.

MVP scope is **B3 (Brazilian) stocks**, priced in BRL. FIIs, ETFs, US equities,
and crypto are explicitly on the roadmap, not in the MVP. The schema is designed
multi-market from the start so adding them later does not require a rewrite.

### Core flow

1. User records buy/sell operations (`quantity`, `price`, `fees`, `date`).
2. Daemon fetches prices and fundamentals from **Yahoo** (primary) with
   **brapi.dev** as fallback.
3. Data is stored in a local **DuckDB** database.
4. The engine computes appreciation, P&L, technical signals, dividend metrics,
   and a 0–100 **opportunity score** per asset.

There is also a non-persisted **search** capability to look up new assets ad hoc;
search results are cached with a TTL but never written into the portfolio.
Portfolio data can be **exported and imported** to move between machines.

## Goals & non-goals

**Goals**
- Track the user's own B3 positions with accurate P&L (realized + unrealized).
- Transparent, configurable opportunity scoring (no black box).
- Fast TUI startup (reads a materialized cache, not live network).
- Resilient price fetching with provider fallback.
- Portable export/import of the ledger.

**Non-goals (MVP)**
- Alerts/notifications (telegram/desktop/terminal) — roadmap.
- FIIs, ETFs, US equities, crypto — roadmap.
- Corporate events (splits/bonificações), brokerage/emolument detail, IR tax
  reporting — roadmap.
- B3 holiday calendar — roadmap (weekend + trading-hours window for now).

## Architecture

### Single binary, three modes

```
wallet daemon                              # long-running: poller + sole DuckDB owner + socket server
wallet tui                                 # ratatui client (auto-starts daemon if the socket is absent)
wallet add / import / export / search ...  # quick CLI commands (also talk over the socket)
```

**Golden rule:** the daemon is the **only** process that opens the DuckDB file.
The TUI and CLI never touch the `.db` directly — they send requests over a Unix
domain socket. This eliminates DuckDB's cross-process single-writer lock conflict
and centralizes all business logic in one place.

### Daemon process (tokio)

- Polling loop, aware of B3 trading hours (idle off-hours, idle on weekends).
- Provider chain (Yahoo → brapi).
- Writes the quote cache and recomputes position snapshots into DuckDB.
- `UnixListener` accepts requests from the TUI/CLI.

### Module map (one crate, boundaries by module)

| Module      | Responsibility                                                                 | I/O      |
|-------------|--------------------------------------------------------------------------------|----------|
| `core`      | Domain types (Trade, Position, Quote) + calculations (avg cost, P&L, signals, score). **Pure.** | None     |
| `storage`   | DuckDB schema, migrations, queries. **Daemon only.**                            | DuckDB   |
| `providers` | `Provider` trait; Yahoo + brapi impls; fallback chain.                          | HTTP     |
| `ipc`       | Request/Response envelope (serde) + socket framing. **Shared** daemon↔client.   | Socket   |
| `daemon`    | Poller + socket server. Wires storage + providers + core.                       | —        |
| `tui`       | ratatui views; ipc client.                                                      | Socket   |
| `main`      | Arg parsing, subcommand dispatch.                                               | —        |

Packaging is a single crate with module boundaries (approach "A"). It can graduate
to a Cargo workspace of separate crates later if the project grows.

### Filesystem locations (XDG defaults)

- DB: `~/.local/share/ticker-wallet/wallet.duckdb`
- Config: `~/.config/ticker-wallet/config.json`
- Socket: `$XDG_RUNTIME_DIR/ticker-wallet.sock`

## Data model (DuckDB)

Money is always `DECIMAL`, never floating point. Positions are **derived** from
the ledger; `position_snapshots` is a discardable materialized cache that is always
reconstructible from `transactions`.

The asset primary key is the composite `(symbol, exchange)`. Tables that reference
an asset carry that pair. In the MVP `exchange = 'BVMF'` is constant, but the
schema is already multi-market.

```sql
assets (                       -- asset metadata; PK is composite
  symbol TEXT, exchange TEXT, name TEXT, kind TEXT, currency TEXT,
  last_seen TIMESTAMP,
  PRIMARY KEY (symbol, exchange) )

transactions (                 -- the ledger; source of truth
  id BIGINT PRIMARY KEY,
  symbol TEXT, exchange TEXT,
  side TEXT,                   -- 'BUY' | 'SELL'
  quantity DECIMAL(18,8), price DECIMAL(18,4),
  fees DECIMAL(18,4) DEFAULT 0,
  executed_at DATE, note TEXT, created_at TIMESTAMP )

quotes (                       -- latest quote snapshot (cache)
  symbol TEXT, exchange TEXT,
  price DECIMAL(18,4), prev_close DECIMAL(18,4),
  day_high DECIMAL(18,4), day_low DECIMAL(18,4),
  currency TEXT, source TEXT, fetched_at TIMESTAMP,
  PRIMARY KEY (symbol, exchange) )

price_history (                -- for SMA50/200, 52w high/low, window returns
  symbol TEXT, exchange TEXT, date DATE,
  open DECIMAL(18,4), high DECIMAL(18,4), low DECIMAL(18,4), close DECIMAL(18,4),
  volume BIGINT,
  PRIMARY KEY (symbol, exchange, date) )

dividends (                    -- proventos / DY / yield-on-cost
  symbol TEXT, exchange TEXT,
  ex_date DATE, pay_date DATE,
  amount_per_share DECIMAL(18,4), source TEXT,
  PRIMARY KEY (symbol, exchange, ex_date) )

position_snapshots (           -- materialized cache of the derived position
  symbol TEXT, exchange TEXT,
  quantity DECIMAL(18,8), avg_cost DECIMAL(18,4),
  invested DECIMAL(18,4), market_value DECIMAL(18,4),
  unrealized_pnl DECIMAL(18,4), unrealized_pnl_pct DECIMAL(9,4),
  realized_pnl DECIMAL(18,4), day_change_pct DECIMAL(9,4),
  score SMALLINT, score_breakdown JSON,   -- sub-scores for the TUI
  computed_at TIMESTAMP,
  PRIMARY KEY (symbol, exchange) )

search_cache (                 -- TTL'd ad-hoc search results; never persisted into the portfolio
  query TEXT PRIMARY KEY, results JSON, fetched_at TIMESTAMP )

schema_version (               -- migrations
  version INT, applied_at TIMESTAMP, checksum TEXT )
```

`schema_version.checksum` is a hash of the migration SQL, used to detect drift
between the recorded version and what the running binary expects.

`quotes` caches the live quote; `position_snapshots` caches the derived position.
The daemon recomputes snapshots via `core` after each poll. The TUI's
`GetPositions` reads from `position_snapshots` so it opens instantly.

## IPC protocol

Unix domain socket, newline-delimited JSON (one JSON object per line), using
`serde` and `tokio::BufReader::lines`.

```jsonc
// Request
{ "id": "uuid", "type": "request", "action": "AddTransaction", "payload": { } }

// Response — success
{ "id": "uuid", "status": "ok", "data": { } }

// Response — error
{ "id": "uuid", "status": "error", "error": { "code": "...", "message": "..." } }
```

- `id` is echoed from request to response so the client can match them on the socket.
- `action` ∈ { `AddTransaction`, `ListTransactions`, `DeleteTransaction`,
  `GetPositions`, `GetPositionDetail`, `RefreshNow`, `Search`, `Import`, `Export`,
  `Ping` }.
- `Search` queries the provider and returns results **without persisting** them to
  the portfolio (only `search_cache` is touched).
- Error `code` ∈ { `NOT_FOUND`, `PROVIDER_DOWN`, `BAD_REQUEST`, `INTERNAL` }.

## Calculations (core)

All calculations live in `core`, are pure, and are unit-tested.

### Position metrics
- Average cost across buys (and reductions on sells).
- Invested amount, current market value.
- Unrealized P&L (absolute + %), realized P&L (from sells), day change %.

### Technical signals
- Distance from 52-week low/high.
- SMA50 / SMA200 and price position relative to them.
- Window returns (7d / 30d / 1y).

### Dividend metrics
- Dividend history, dividend yield, yield on cost.

### Opportunity score (0–100)

Sub-scores are each normalized to 0–100, then combined as a weighted average.
"Opportunity" means a good moment to **buy / add to** the position. Weights live
in `config.json` and are editable; the TUI shows the breakdown (transparent).

| Sub-score                          | Signal                                                       | Default weight |
|------------------------------------|--------------------------------------------------------------|----------------|
| Proximity to 52-week low           | closer to the low = cheaper                                  | 25             |
| Below SMA200 / SMA50               | price < moving averages = discount                           | 20             |
| 30-day drawdown                    | recent drop = entry window                                   | 15             |
| Dividend yield                     | higher DY = income                                           | 20             |
| Distance from cost basis vs trend  | how far below your average cost, weighted by trend           | 20             |

**Distance from cost basis vs trend:** measures how far below your average cost
the price is, weighted by trend (price vs SMA50 + its slope). Underwater + trend
turning up = strong averaging-down opportunity. Underwater + trend still falling =
lower score (falling-knife guard).

```
score = Σ(sub_i × weight_i) / Σ(weight_i)
```

## Providers

```rust
trait Provider {            // async, reqwest + serde
    fn quote(symbol) -> Quote;
    fn history(symbol, range) -> Vec<Candle>;
    fn dividends(symbol) -> Vec<Dividend>;
    fn search(query) -> Vec<Asset>;
}
```

- **Yahoo (primary):** `query1.finance.yahoo.com` (chart + quoteSummary endpoints).
  B3 symbols use the `.SA` suffix. No API key (undocumented endpoint).
- **brapi (fallback):** `brapi.dev`, token from config. `/quote`,
  `?dividends=true`, `/available` for search.
- **Per-call fallback chain:** try the primary; on error/timeout/empty result,
  fall through to the fallback. Record `source` in `quotes`. Backoff + minimum
  request interval to respect rate limits.

## Polling / trading hours

- B3 session, weekdays ~10:00–18:00 BRT → poll every `N` seconds (config,
  default 60s). Off-hours: idle; refresh history + dividends once per day.
- `RefreshNow` always works on demand.
- Daemon boot: run migrations, then backfill history + dividends for held tickers.
- B3 holiday calendar is roadmap; for now only weekend + the hours window gate polling.

## Error handling

- **Providers:** if all fail, keep the last cached quote and mark it **stale**
  (the TUI shows a staleness indicator). The poller never crashes — it logs and
  continues.
- **IPC:** structured `Error { code, message }` with the codes listed above.
- **DB:** a migration checksum mismatch refuses to start with a clear message.
  A stale socket (daemon died) is detected by the TUI on connect failure; it
  removes the stale socket and restarts the daemon.
- **Validation (core):** rejects negative quantity, selling more than held, and
  unknown tickers.
- DB writes are transactional; snapshot recomputation is idempotent.

## Testing

- **core:** pure unit tests (TDD) — average cost across buys/sells, realized vs
  unrealized P&L, each sub-score, the composite score, and edge cases (zero
  quantity, fully sold).
- **providers:** HTTP mocked (wiremock) with real Yahoo/brapi response fixtures;
  exercises the fallback chain.
- **ipc:** serde round-trip of the envelope; socket integration test (spawn
  daemon, send request, assert response).
- **storage:** temp DuckDB, apply migration + checksum, query correctness.
- **e2e smoke:** temp dirs, daemon up, `add` a transaction via CLI, `GetPositions`
  returns the expected result.

## Roadmap (out of MVP)

- Alerts (telegram / desktop / terminal).
- FIIs, ETFs, US equities, crypto.
- Corporate events (splits/bonificações), detailed brokerage/emoluments, IR tax.
- B3 holiday calendar.
- Graduate packaging from a single crate to a Cargo workspace (approach A → B).
