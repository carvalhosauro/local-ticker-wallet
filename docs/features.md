# Features

## Modes of operation

`ltw` is a single binary with three roles:

| Mode | Command | Role |
|------|---------|------|
| Daemon | `ltw daemon` | Owns SQLite, polls market data, serves IPC |
| TUI | `ltw tui` | Full-screen terminal UI (ratatui) |
| CLI | `ltw add`, `list`, … | Thin JSON clients over the socket |

The daemon is the **only** process that opens the database. TUI and CLI never touch `wallet.db` directly.

## Portfolio ledger

Record buy and sell operations with quantity, price, fees, date, and an optional note.

```bash
ltw add PETR4 100 28.50 --date 2026-01-02 --note "first buy"
ltw add PETR4 50 30.00 --side SELL --date 2026-06-01 --fees 2.50
ltw list
ltw list --symbol PETR4
ltw delete 42          # transaction id from list output
```

Positions are **derived** from the ledger. The daemon materializes `position_snapshots` after each change or poll so reads stay fast.

### Computed metrics

For each held asset the engine computes:

- Average cost (across buys, reduced on sells)
- Invested amount and current market value
- Unrealized P&L (absolute and %)
- Realized P&L from sells
- Day change %
- Technical signals: 52-week high/low distance, SMA50/200, 7d / 30d / 1y returns
- Dividend metrics: history, trailing yield, yield on cost

Money is stored and calculated with `rust_decimal` — never floating point.

## Opportunity score

Each position gets a **0–100 opportunity score**: higher means a better moment to **add to** the position (not a buy/sell recommendation).

Sub-scores (each normalized 0–100) are combined as a weighted average:

| Sub-score | Signal | Default weight |
|-----------|--------|----------------|
| Proximity to 52-week low | Closer to the low = cheaper entry | 25 |
| Below SMA50 / SMA200 | Price below moving averages | 20 |
| 30-day drawdown | Recent drop = potential entry window | 15 |
| Dividend yield | Higher trailing yield | 20 |
| Cost basis vs trend | Underwater + uptrend = averaging-down opportunity | 20 |

Weights are editable in `config.json`. The TUI detail screen shows the full breakdown.

## Market data

| Provider | Role | Notes |
|----------|------|-------|
| Yahoo Finance | Primary | `query1.finance.yahoo.com`; B3 tickers use `.SA` suffix |
| brapi.dev | Fallback | Optional token in config |

The poller respects B3 trading hours (weekdays ~10:00–18:00 BRT). Off-hours it idles; history and dividends refresh once per day. `ltw refresh` forces an immediate update.

```bash
ltw refresh              # all held symbols
ltw refresh --symbol PETR4
```

If all providers fail, the last cached quote is kept and marked stale in the TUI.

## Search

Look up assets without adding them to the portfolio:

```bash
ltw search petro
```

Results are cached with a TTL in `search_cache` only — search never writes to the ledger.

## Export & import

Move your ledger between machines as CSV:

```bash
ltw export portfolio.csv
ltw import portfolio.csv
```

Format (header required):

```csv
symbol,exchange,side,quantity,price,fees,executed_at,note
PETR4,BVMF,BUY,100,28.50,0,2026-01-02,first buy
```

Import recomputes position snapshots for every touched asset.

## Terminal UI

`ltw tui` provides:

- **Portfolio** — sortable position list with scores and P&L
- **Detail** — metrics, score breakdown, braille price chart
- **Ledger** — transaction history with add/delete overlays
- **Search** — provider lookup with preview

Keyboard-driven navigation; locale follows `config.json` (`pt-BR` default, `en` available).

## IPC actions

Clients send newline-delimited JSON over a Unix domain socket:

`AddTransaction`, `ListTransactions`, `DeleteTransaction`, `GetPositions`, `GetPositionDetail`, `RefreshNow`, `Search`, `Import`, `Export`, `Ping`

See [Architecture](architecture.md#ipc-protocol) for the wire format.

## Roadmap (not in MVP)

- Alerts (telegram / desktop / terminal)
- FIIs, ETFs, US equities, crypto
- Corporate events (splits, bonificações), brokerage detail, IR tax helpers
- Full B3 holiday calendar
