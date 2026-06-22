# local-ticker-wallet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust TUI personal stock wallet for B3 assets, backed by a background daemon that polls market data into a local DuckDB cache and computes per-asset P&L, technical signals, and a 0–100 opportunity score.

**Architecture:** Single crate, three run modes (`daemon`, `tui`, CLI subcommands). The daemon is the **only** process that opens DuckDB; the TUI and CLI are thin clients that talk to it over a Unix-domain socket using a newline-delimited JSON envelope. All math lives in a pure `core` module that is unit-tested in isolation.

**Tech Stack:** Rust 2021, `tokio` (async daemon + socket), `ratatui` + `crossterm` (TUI), `duckdb` (bundled), `reqwest` + `serde`/`serde_json` (HTTP + protocol), `rust_decimal` (exact money), `chrono` + `chrono-tz` (dates/B3 hours), `clap` (CLI), `uuid` (request ids), `sha2` (migration checksums), `directories` (XDG paths), `anyhow`/`thiserror` (errors). Dev: `wiremock`, `tempfile`.

## Global Constraints

- Rust edition 2021, MSRV 1.75+.
- **Money is always `rust_decimal::Decimal`, never `f32`/`f64`.** DuckDB DECIMAL columns are bound as strings (`CAST(? AS DECIMAL(...))`) and read back via `CAST(col AS VARCHAR)` then `Decimal::from_str`. No float anywhere in the money path.
- Asset identity is the composite `(symbol, exchange)`. MVP holds `exchange = "BVMF"` constant but never hard-codes it away.
- The daemon is the sole DuckDB opener. TUI/CLI never open the `.db`; they use the socket.
- IPC is newline-delimited JSON. Envelope: request `{ id, type:"request", action, payload }`; response `{ id, status:"ok", data }` or `{ id, status:"error", error:{ code, message } }`. `id` echoes request→response. Error codes ∈ `{ NOT_FOUND, PROVIDER_DOWN, BAD_REQUEST, INTERNAL }`.
- Provider order: Yahoo (primary) → brapi (fallback), per call. `source` recorded on every quote.
- XDG paths: DB `~/.local/share/local-ticker-wallet/wallet.duckdb`, config `~/.config/local-ticker-wallet/config.json`, socket `$XDG_RUNTIME_DIR/local-ticker-wallet.sock`.
- TDD: write the failing test, watch it fail, implement minimally, watch it pass, commit. Small commits per task.

---

### Task 1: Project scaffold, paths, config

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/paths.rs`, `src/config.rs`, `.gitignore`
- Test: inline `#[cfg(test)]` in `src/config.rs`

**Interfaces:**
- Produces: `paths::data_db() -> PathBuf`, `paths::config_file() -> PathBuf`, `paths::socket_path() -> PathBuf`.
- Produces: `config::Config { brapi_token: Option<String>, poll_interval_secs: u64, score_weights: ScoreWeights }`, `Config::load() -> anyhow::Result<Config>`, `Config::default()`.
- Produces: `config::ScoreWeights { proximity_low: u32, below_sma: u32, drawdown: u32, dividend_yield: u32, cost_vs_trend: u32 }` (defaults 25/20/15/20/20).

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "local-ticker-wallet"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[[bin]]
name = "ltw"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "macros", "time", "process", "signal"] }
ratatui = "0.28"
crossterm = "0.28"
duckdb = { version = "1.1", features = ["bundled"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rust_decimal = { version = "1", features = ["serde-with-str"] }
rust_decimal_macros = "1"
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
clap = { version = "4", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
sha2 = "0.10"
directories = "5"
anyhow = "1"
thiserror = "1"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

- [ ] **Step 2: Create `.gitignore`**

```
/target
*.duckdb
*.duckdb.wal
```

- [ ] **Step 3: Create `src/paths.rs`**

```rust
use std::path::PathBuf;
use directories::ProjectDirs;

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("dev", "local-ticker-wallet", "local-ticker-wallet")
        .expect("cannot resolve home directory")
}

pub fn data_db() -> PathBuf {
    let dir = project_dirs().data_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("wallet.duckdb")
}

pub fn config_file() -> PathBuf {
    let dir = project_dirs().config_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

pub fn socket_path() -> PathBuf {
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(rt).join("local-ticker-wallet.sock");
    }
    std::env::temp_dir().join("local-ticker-wallet.sock")
}
```

- [ ] **Step 4: Write the failing test in `src/config.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub proximity_low: u32,
    pub below_sma: u32,
    pub drawdown: u32,
    pub dividend_yield: u32,
    pub cost_vs_trend: u32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self { proximity_low: 25, below_sma: 20, drawdown: 15, dividend_yield: 20, cost_vs_trend: 20 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub brapi_token: Option<String>,
    pub poll_interval_secs: u64,
    pub score_weights: ScoreWeights,
}

impl Default for Config {
    fn default() -> Self {
        Self { brapi_token: None, poll_interval_secs: 60, score_weights: ScoreWeights::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrips_through_json() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
        assert_eq!(back.poll_interval_secs, 60);
        assert_eq!(back.score_weights.proximity_low, 25);
    }

    #[test]
    fn partial_json_fills_defaults() {
        let back: Config = serde_json::from_str("{\"poll_interval_secs\": 30}").unwrap();
        assert_eq!(back.poll_interval_secs, 30);
        assert_eq!(back.score_weights.dividend_yield, 20);
    }
}
```

- [ ] **Step 5: Add `Config::load` below the structs**

```rust
impl Config {
    pub fn load() -> anyhow::Result<Config> {
        let path = crate::paths::config_file();
        if !path.exists() {
            let cfg = Config::default();
            std::fs::write(&path, serde_json::to_string_pretty(&cfg)?)?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&text)?)
    }
}
```

- [ ] **Step 6: Create `src/lib.rs`**

```rust
pub mod paths;
pub mod config;
```

- [ ] **Step 7: Create a minimal `src/main.rs` (replaced in Task 16)**

```rust
fn main() {
    println!("local-ticker-wallet (scaffold)");
}
```

- [ ] **Step 8: Build and test**

Run: `cargo test`
Expected: PASS (2 tests in `config`).

- [ ] **Step 9: Commit**

```bash
git init && git add -A && git commit -m "chore: scaffold local-ticker-wallet crate, paths, config"
```

---

### Task 2: Core domain types

**Files:**
- Create: `src/core/mod.rs`, `src/core/types.rs`
- Modify: `src/lib.rs` (add `pub mod core;`)
- Test: inline in `src/core/types.rs`

**Interfaces:**
- Produces: `AssetId { symbol: String, exchange: String }`, `Side { Buy, Sell }`, `Trade { id, asset, side, quantity, price, fees, executed_at, note }`, `Quote { asset, price, prev_close, day_high, day_low, currency, source, fetched_at }`, `Candle { date, open, high, low, close, volume }`, `Dividend { asset, ex_date, pay_date, amount_per_share }`, `Asset { id, name, kind, currency }`.
- All money/quantity fields are `rust_decimal::Decimal`; dates are `chrono::NaiveDate`; timestamps `chrono::NaiveDateTime`.

- [ ] **Step 1: Create `src/core/mod.rs`**

```rust
pub mod types;
pub mod pnl;
pub mod signals;
pub mod score;
```

- [ ] **Step 2: Write the failing test + types in `src/core/types.rs`**

```rust
use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId {
    pub symbol: String,
    pub exchange: String,
}

impl AssetId {
    pub fn b3(symbol: &str) -> Self {
        Self { symbol: symbol.to_uppercase(), exchange: "BVMF".to_string() }
    }
    /// Yahoo ticker for B3 symbols, e.g. PETR4 -> PETR4.SA
    pub fn yahoo_ticker(&self) -> String {
        if self.exchange == "BVMF" { format!("{}.SA", self.symbol) } else { self.symbol.clone() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side { Buy, Sell }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub id: i64,
    pub asset: AssetId,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fees: Decimal,
    pub executed_at: NaiveDate,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub asset: AssetId,
    pub price: Decimal,
    pub prev_close: Decimal,
    pub day_high: Decimal,
    pub day_low: Decimal,
    pub currency: String,
    pub source: String,
    pub fetched_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub date: NaiveDate,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dividend {
    pub asset: AssetId,
    pub ex_date: NaiveDate,
    pub pay_date: Option<NaiveDate>,
    pub amount_per_share: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub name: String,
    pub kind: String,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b3_asset_builds_yahoo_ticker() {
        let a = AssetId::b3("petr4");
        assert_eq!(a.symbol, "PETR4");
        assert_eq!(a.exchange, "BVMF");
        assert_eq!(a.yahoo_ticker(), "PETR4.SA");
    }

    #[test]
    fn side_serializes_uppercase() {
        assert_eq!(serde_json::to_string(&Side::Buy).unwrap(), "\"BUY\"");
    }
}
```

- [ ] **Step 3: Register module in `src/lib.rs`**

Add line: `pub mod core;`

- [ ] **Step 4: Run tests**

Run: `cargo test core::types`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): domain types (AssetId, Trade, Quote, Candle, Dividend)"
```

---

### Task 3: Position from ledger (avg cost + realized P&L)

**Files:**
- Create: `src/core/pnl.rs`
- Test: inline in `src/core/pnl.rs`

**Interfaces:**
- Consumes: `core::types::{Trade, Side, AssetId}`.
- Produces: `pnl::Position { asset: AssetId, quantity: Decimal, avg_cost: Decimal, invested: Decimal, realized_pnl: Decimal }`, `pnl::PnlError` (`thiserror`), `Position::from_trades(asset: &AssetId, trades: &[Trade]) -> Result<Position, PnlError>`.
- Method: weighted-average cost. Buy folds fees into cost basis; sell realizes `qty*(price - avg_cost) - fees`, leaves `avg_cost` unchanged. Selling more than held → `PnlError::Oversell`.

- [ ] **Step 1: Write the failing tests in `src/core/pnl.rs`**

```rust
use crate::core::types::{AssetId, Side, Trade};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PnlError {
    #[error("selling {sell} but only {held} held of {symbol}")]
    Oversell { symbol: String, sell: Decimal, held: Decimal },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub asset: AssetId,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub invested: Decimal,
    pub realized_pnl: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn trade(id: i64, side: Side, qty: &str, price: &str, fees: &str, day: u32) -> Trade {
        Trade {
            id,
            asset: AssetId::b3("PETR4"),
            side,
            quantity: qty.parse().unwrap(),
            price: price.parse().unwrap(),
            fees: fees.parse().unwrap(),
            executed_at: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            note: None,
        }
    }

    #[test]
    fn two_buys_weighted_average() {
        let asset = AssetId::b3("PETR4");
        let trades = vec![
            trade(1, Side::Buy, "100", "10.00", "0", 1),
            trade(2, Side::Buy, "100", "12.00", "0", 2),
        ];
        let p = Position::from_trades(&asset, &trades).unwrap();
        assert_eq!(p.quantity, dec!(200));
        assert_eq!(p.avg_cost, dec!(11.00));
        assert_eq!(p.invested, dec!(2200.00));
        assert_eq!(p.realized_pnl, dec!(0));
    }

    #[test]
    fn buy_fees_fold_into_cost() {
        let asset = AssetId::b3("PETR4");
        let trades = vec![trade(1, Side::Buy, "100", "10.00", "5.00", 1)];
        let p = Position::from_trades(&asset, &trades).unwrap();
        // (1000 + 5) / 100
        assert_eq!(p.avg_cost, dec!(10.05));
    }

    #[test]
    fn sell_realizes_pnl_keeps_avg_cost() {
        let asset = AssetId::b3("PETR4");
        let trades = vec![
            trade(1, Side::Buy, "100", "10.00", "0", 1),
            trade(2, Side::Sell, "40", "15.00", "2.00", 2),
        ];
        let p = Position::from_trades(&asset, &trades).unwrap();
        assert_eq!(p.quantity, dec!(60));
        assert_eq!(p.avg_cost, dec!(10.00));
        // 40*(15-10) - 2
        assert_eq!(p.realized_pnl, dec!(198.00));
    }

    #[test]
    fn oversell_errors() {
        let asset = AssetId::b3("PETR4");
        let trades = vec![
            trade(1, Side::Buy, "10", "10.00", "0", 1),
            trade(2, Side::Sell, "20", "11.00", "0", 2),
        ];
        assert!(matches!(Position::from_trades(&asset, &trades), Err(PnlError::Oversell { .. })));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test core::pnl`
Expected: FAIL — `Position::from_trades` not found.

- [ ] **Step 3: Implement `from_trades` above the `tests` module**

```rust
impl Position {
    pub fn from_trades(asset: &AssetId, trades: &[Trade]) -> Result<Position, PnlError> {
        let mut sorted: Vec<&Trade> = trades.iter().collect();
        sorted.sort_by(|a, b| a.executed_at.cmp(&b.executed_at).then(a.id.cmp(&b.id)));

        let mut qty = Decimal::ZERO;
        let mut avg_cost = Decimal::ZERO;
        let mut realized = Decimal::ZERO;

        for t in sorted {
            match t.side {
                Side::Buy => {
                    let prior_cost = qty * avg_cost;
                    let new_cost = prior_cost + t.quantity * t.price + t.fees;
                    qty += t.quantity;
                    avg_cost = if qty.is_zero() { Decimal::ZERO } else { new_cost / qty };
                }
                Side::Sell => {
                    if t.quantity > qty {
                        return Err(PnlError::Oversell {
                            symbol: asset.symbol.clone(),
                            sell: t.quantity,
                            held: qty,
                        });
                    }
                    realized += t.quantity * (t.price - avg_cost) - t.fees;
                    qty -= t.quantity;
                }
            }
        }

        Ok(Position {
            asset: asset.clone(),
            quantity: qty,
            avg_cost,
            invested: qty * avg_cost,
            realized_pnl: realized,
        })
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test core::pnl`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): Position::from_trades with weighted-average cost and realized P&L"
```

---

### Task 4: Valuation (unrealized P&L + day change)

**Files:**
- Modify: `src/core/pnl.rs` (append `Valuation`)
- Test: inline in `src/core/pnl.rs`

**Interfaces:**
- Consumes: `pnl::Position`, `core::types::Quote`.
- Produces: `pnl::Valuation { market_value, unrealized_pnl, unrealized_pnl_pct, day_change_pct }` and `Valuation::compute(position: &Position, quote: &Quote) -> Valuation`. Percentages are `Decimal` in percent units (e.g. `5.00` = +5%). Zero-cost/zero-prev-close guarded to `Decimal::ZERO`.

- [ ] **Step 1: Write the failing test (append inside the existing `tests` module)**

```rust
    #[test]
    fn valuation_basic() {
        use crate::core::types::Quote;
        use chrono::NaiveDate;
        let asset = AssetId::b3("PETR4");
        let pos = Position {
            asset: asset.clone(),
            quantity: dec!(100),
            avg_cost: dec!(10.00),
            invested: dec!(1000.00),
            realized_pnl: dec!(0),
        };
        let quote = Quote {
            asset,
            price: dec!(12.00),
            prev_close: dec!(11.00),
            day_high: dec!(12.50),
            day_low: dec!(10.90),
            currency: "BRL".into(),
            source: "test".into(),
            fetched_at: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap().and_hms_opt(12, 0, 0).unwrap(),
        };
        let v = Valuation::compute(&pos, &quote);
        assert_eq!(v.market_value, dec!(1200.00));
        assert_eq!(v.unrealized_pnl, dec!(200.00));
        assert_eq!(v.unrealized_pnl_pct, dec!(20));   // 200/1000 * 100
        // (12 - 11)/11 * 100
        assert_eq!(v.day_change_pct.round_dp(4), dec!(9.0909));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test core::pnl::tests::valuation_basic`
Expected: FAIL — `Valuation` not found.

- [ ] **Step 3: Implement `Valuation` (append after the `Position` impl)**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Valuation {
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub day_change_pct: Decimal,
}

impl Valuation {
    pub fn compute(position: &super::pnl::Position, quote: &crate::core::types::Quote) -> Valuation {
        use rust_decimal::Decimal;
        let hundred = Decimal::from(100);
        let market_value = position.quantity * quote.price;
        let unrealized_pnl = market_value - position.invested;
        let unrealized_pnl_pct = if position.invested.is_zero() {
            Decimal::ZERO
        } else {
            unrealized_pnl / position.invested * hundred
        };
        let day_change_pct = if quote.prev_close.is_zero() {
            Decimal::ZERO
        } else {
            (quote.price - quote.prev_close) / quote.prev_close * hundred
        };
        Valuation { market_value, unrealized_pnl, unrealized_pnl_pct, day_change_pct }
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test core::pnl`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): Valuation with unrealized P&L and day change"
```

---

### Task 5: Technical signals

**Files:**
- Create: `src/core/signals.rs`
- Test: inline in `src/core/signals.rs`

**Interfaces:**
- Consumes: `core::types::Candle`.
- Produces (all take a chronologically-sorted `&[Candle]`, newest last):
  - `sma(candles, n: usize) -> Option<Decimal>` — average of last `n` closes, `None` if fewer than `n`.
  - `high_low_52w(candles) -> Option<(Decimal, Decimal)>` — (low, high) over last 252 candles.
  - `drawdown_pct(candles, days: usize) -> Option<Decimal>` — percent drop from the max close in the last `days` to the latest close (negative = down).
  - `window_return_pct(candles, days: usize) -> Option<Decimal>` — percent change from close `days` ago to latest.

- [ ] **Step 1: Write the failing tests in `src/core/signals.rs`**

```rust
use crate::core::types::Candle;
use rust_decimal::Decimal;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn candle(day: u32, close: &str) -> Candle {
        let c: Decimal = close.parse().unwrap();
        Candle {
            date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            open: c, high: c, low: c, close: c, volume: 1,
        }
    }

    fn series(closes: &[&str]) -> Vec<Candle> {
        closes.iter().enumerate().map(|(i, c)| candle((i + 1) as u32, c)).collect()
    }

    #[test]
    fn sma_last_n() {
        let s = series(&["10", "20", "30", "40"]);
        assert_eq!(sma(&s, 2), Some(dec!(35)));   // (30+40)/2
        assert_eq!(sma(&s, 4), Some(dec!(25)));
        assert_eq!(sma(&s, 5), None);
    }

    #[test]
    fn high_low_52w_picks_extremes() {
        let s = series(&["10", "5", "30", "20"]);
        assert_eq!(high_low_52w(&s), Some((dec!(5), dec!(30))));
    }

    #[test]
    fn drawdown_from_recent_peak() {
        // peak 40, latest 30 over the window -> -25%
        let s = series(&["10", "40", "35", "30"]);
        assert_eq!(drawdown_pct(&s, 4), Some(dec!(-25)));
    }

    #[test]
    fn window_return() {
        // from close 3 days ago (10) to latest (12) -> +20%
        let s = series(&["8", "10", "11", "12"]);
        assert_eq!(window_return_pct(&s, 3), Some(dec!(20)));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test core::signals`
Expected: FAIL — functions not found.

- [ ] **Step 3: Implement the functions (above `tests`)**

```rust
fn last_n(candles: &[Candle], n: usize) -> Option<&[Candle]> {
    if candles.len() < n || n == 0 { None } else { Some(&candles[candles.len() - n..]) }
}

pub fn sma(candles: &[Candle], n: usize) -> Option<Decimal> {
    let window = last_n(candles, n)?;
    let sum: Decimal = window.iter().map(|c| c.close).sum();
    Some(sum / Decimal::from(n))
}

pub fn high_low_52w(candles: &[Candle]) -> Option<(Decimal, Decimal)> {
    let n = candles.len().min(252);
    let window = last_n(candles, n)?;
    let low = window.iter().map(|c| c.low).min()?;
    let high = window.iter().map(|c| c.high).max()?;
    Some((low, high))
}

pub fn drawdown_pct(candles: &[Candle], days: usize) -> Option<Decimal> {
    let n = candles.len().min(days);
    let window = last_n(candles, n)?;
    let peak = window.iter().map(|c| c.close).max()?;
    let latest = window.last()?.close;
    if peak.is_zero() { return Some(Decimal::ZERO); }
    Some((latest - peak) / peak * Decimal::from(100))
}

pub fn window_return_pct(candles: &[Candle], days: usize) -> Option<Decimal> {
    if candles.len() <= days { return None; }
    let start = candles[candles.len() - 1 - days].close;
    let latest = candles[candles.len() - 1].close;
    if start.is_zero() { return Some(Decimal::ZERO); }
    Some((latest - start) / start * Decimal::from(100))
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test core::signals`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): technical signals (SMA, 52w hi/lo, drawdown, window return)"
```

---

### Task 6: Opportunity score

**Files:**
- Create: `src/core/score.rs`
- Test: inline in `src/core/score.rs`

**Interfaces:**
- Consumes: `pnl::Position`, `core::types::{Quote, Candle, Dividend}`, `config::ScoreWeights`, `core::signals`.
- Produces: `score::ScoreBreakdown { proximity_low: Decimal, below_sma: Decimal, drawdown: Decimal, dividend_yield: Decimal, cost_vs_trend: Decimal, total: u8 }` and `score::compute(position, quote, candles, dividends, weights) -> ScoreBreakdown`.
- Each sub-score is a `Decimal` in 0–100; `total` is the weighted average rounded to `u8` (0–100). Missing data → that sub-score is 0 and its weight still counts (conservative).

- [ ] **Step 1: Write the failing test in `src/core/score.rs`**

```rust
use crate::config::ScoreWeights;
use crate::core::pnl::Position;
use crate::core::signals;
use crate::core::types::{Candle, Dividend, Quote};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub proximity_low: Decimal,
    pub below_sma: Decimal,
    pub drawdown: Decimal,
    pub dividend_yield: Decimal,
    pub cost_vs_trend: Decimal,
    pub total: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::AssetId;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn candle(day: u32, close: &str) -> Candle {
        let c: Decimal = close.parse().unwrap();
        Candle { date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(), open: c, high: c, low: c, close: c, volume: 1 }
    }

    #[test]
    fn cheap_stock_near_low_scores_high() {
        let asset = AssetId::b3("PETR4");
        // Series declining to its low; latest == 52w low.
        let candles: Vec<Candle> = (1..=30).map(|d| candle(d, &format!("{}", 40 - d))).collect();
        let pos = Position { asset: asset.clone(), quantity: dec!(100), avg_cost: dec!(30.00), invested: dec!(3000.00), realized_pnl: dec!(0) };
        let quote = Quote { asset, price: dec!(10.00), prev_close: dec!(11.00), day_high: dec!(11), day_low: dec!(10), currency: "BRL".into(), source: "t".into(), fetched_at: NaiveDate::from_ymd_opt(2026,2,1).unwrap().and_hms_opt(0,0,0).unwrap() };
        let divs: Vec<Dividend> = vec![];
        let b = compute(&pos, &quote, &candles, &divs, &ScoreWeights::default());
        assert!(b.proximity_low >= dec!(90), "near low should score high, got {}", b.proximity_low);
        assert!(b.total >= 50, "underwater + cheap should be an opportunity, got {}", b.total);
    }

    #[test]
    fn total_is_weighted_average_bounded_0_100() {
        let asset = AssetId::b3("VALE3");
        let candles: Vec<Candle> = (1..=30).map(|d| candle(d, "50")).collect();
        let pos = Position { asset: asset.clone(), quantity: dec!(10), avg_cost: dec!(50), invested: dec!(500), realized_pnl: dec!(0) };
        let quote = Quote { asset, price: dec!(50), prev_close: dec!(50), day_high: dec!(50), day_low: dec!(50), currency: "BRL".into(), source: "t".into(), fetched_at: NaiveDate::from_ymd_opt(2026,2,1).unwrap().and_hms_opt(0,0,0).unwrap() };
        let b = compute(&pos, &quote, &candles, &[], &ScoreWeights::default());
        assert!(b.total <= 100);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test core::score`
Expected: FAIL — `compute` not found.

- [ ] **Step 3: Implement `compute` and helpers (above `tests`)**

```rust
fn clamp_0_100(v: Decimal) -> Decimal {
    let zero = Decimal::ZERO;
    let hundred = Decimal::from(100);
    if v < zero { zero } else if v > hundred { hundred } else { v }
}

/// 100 when price sits at the 52w low, 0 at the 52w high.
fn proximity_low_score(quote: &Quote, candles: &[Candle]) -> Decimal {
    match signals::high_low_52w(candles) {
        Some((low, high)) if high > low => {
            let pct = (high - quote.price) / (high - low) * Decimal::from(100);
            clamp_0_100(pct)
        }
        _ => Decimal::ZERO,
    }
}

/// Rewards price below SMA50 and SMA200; up to 50 points each.
fn below_sma_score(quote: &Quote, candles: &[Candle]) -> Decimal {
    let mut s = Decimal::ZERO;
    for (n, weight) in [(50usize, 50i64), (200usize, 50i64)] {
        if let Some(sma) = signals::sma(candles, n) {
            if !sma.is_zero() && quote.price < sma {
                let depth = (sma - quote.price) / sma * Decimal::from(100); // % below
                s += clamp_0_100(depth * Decimal::from(5)).min(Decimal::from(weight));
            }
        }
    }
    clamp_0_100(s)
}

/// Recent drop magnitude (30d) mapped to 0..100 (a -20% drawdown -> 100).
fn drawdown_score(candles: &[Candle]) -> Decimal {
    match signals::drawdown_pct(candles, 30) {
        Some(dd) if dd < Decimal::ZERO => clamp_0_100(-dd * Decimal::from(5)),
        _ => Decimal::ZERO,
    }
}

/// Trailing dividend yield on current price, scaled (10% yield -> 100).
fn dividend_yield_score(quote: &Quote, dividends: &[Dividend]) -> Decimal {
    if quote.price.is_zero() { return Decimal::ZERO; }
    let ttm: Decimal = dividends.iter().map(|d| d.amount_per_share).sum();
    let dy = ttm / quote.price * Decimal::from(100);
    clamp_0_100(dy * Decimal::from(10))
}

/// How far price is below avg cost, gated by trend (price vs SMA50).
/// Underwater + price >= SMA50 (recovering) -> full credit; underwater in a
/// downtrend -> halved (falling-knife guard). Above cost -> 0.
fn cost_vs_trend_score(position: &Position, quote: &Quote, candles: &[Candle]) -> Decimal {
    if position.avg_cost.is_zero() || quote.price >= position.avg_cost {
        return Decimal::ZERO;
    }
    let below = (position.avg_cost - quote.price) / position.avg_cost * Decimal::from(100);
    let base = clamp_0_100(below * Decimal::from(5));
    let recovering = signals::sma(candles, 50).map(|s| quote.price >= s).unwrap_or(false);
    if recovering { base } else { base / Decimal::from(2) }
}

pub fn compute(
    position: &Position,
    quote: &Quote,
    candles: &[Candle],
    dividends: &[Dividend],
    weights: &ScoreWeights,
) -> ScoreBreakdown {
    let proximity_low = proximity_low_score(quote, candles);
    let below_sma = below_sma_score(quote, candles);
    let drawdown = drawdown_score(candles);
    let dividend_yield = dividend_yield_score(quote, dividends);
    let cost_vs_trend = cost_vs_trend_score(position, quote, candles);

    let w = weights;
    let total_weight = Decimal::from(w.proximity_low + w.below_sma + w.drawdown + w.dividend_yield + w.cost_vs_trend);
    let weighted = proximity_low * Decimal::from(w.proximity_low)
        + below_sma * Decimal::from(w.below_sma)
        + drawdown * Decimal::from(w.drawdown)
        + dividend_yield * Decimal::from(w.dividend_yield)
        + cost_vs_trend * Decimal::from(w.cost_vs_trend);
    let total = if total_weight.is_zero() { Decimal::ZERO } else { weighted / total_weight };
    use rust_decimal::prelude::ToPrimitive;
    let total = clamp_0_100(total).round().to_u8().unwrap_or(0);

    ScoreBreakdown { proximity_low, below_sma, drawdown, dividend_yield, cost_vs_trend, total }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test core::score`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): opportunity score with transparent weighted sub-scores"
```

---

### Task 7: Storage — schema, migrations, checksum

**Files:**
- Create: `src/storage/mod.rs`, `src/storage/schema.rs`, `src/storage/db.rs`
- Modify: `src/lib.rs` (add `pub mod storage;`)
- Test: inline in `src/storage/db.rs`

**Interfaces:**
- Produces: `storage::db::Db` wrapping `duckdb::Connection`; `Db::open(path: &Path) -> anyhow::Result<Db>` runs migrations on open; `Db::open_in_memory() -> anyhow::Result<Db>` for tests; `Db::schema_version() -> anyhow::Result<i32>`.
- Produces: `schema::MIGRATIONS: &[(i32, &str)]` (version, SQL) and `schema::checksum(sql: &str) -> String` (hex SHA-256).
- Migration runner records `(version, applied_at, checksum)` and refuses to run if a recorded checksum differs from the embedded migration's checksum (`anyhow::bail!` with a clear message).

- [ ] **Step 1: Create `src/storage/mod.rs`**

```rust
pub mod schema;
pub mod db;
pub mod queries;
```

- [ ] **Step 2: Create `src/storage/schema.rs`**

```rust
use sha2::{Digest, Sha256};

pub fn checksum(sql: &str) -> String {
    let mut h = Sha256::new();
    h.update(sql.as_bytes());
    format!("{:x}", h.finalize())
}

pub const MIGRATIONS: &[(i32, &str)] = &[(
    1,
    r#"
    CREATE TABLE assets (
        symbol TEXT, exchange TEXT, name TEXT, kind TEXT, currency TEXT,
        last_seen TIMESTAMP,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE SEQUENCE tx_id_seq START 1;
    CREATE TABLE transactions (
        id BIGINT PRIMARY KEY DEFAULT nextval('tx_id_seq'),
        symbol TEXT, exchange TEXT, side TEXT,
        quantity DECIMAL(18,8), price DECIMAL(18,4), fees DECIMAL(18,4) DEFAULT 0,
        executed_at DATE, note TEXT, created_at TIMESTAMP DEFAULT now()
    );
    CREATE TABLE quotes (
        symbol TEXT, exchange TEXT,
        price DECIMAL(18,4), prev_close DECIMAL(18,4),
        day_high DECIMAL(18,4), day_low DECIMAL(18,4),
        currency TEXT, source TEXT, fetched_at TIMESTAMP,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE TABLE price_history (
        symbol TEXT, exchange TEXT, date DATE,
        open DECIMAL(18,4), high DECIMAL(18,4), low DECIMAL(18,4), close DECIMAL(18,4),
        volume BIGINT,
        PRIMARY KEY (symbol, exchange, date)
    );
    CREATE TABLE dividends (
        symbol TEXT, exchange TEXT, ex_date DATE, pay_date DATE,
        amount_per_share DECIMAL(18,4), source TEXT,
        PRIMARY KEY (symbol, exchange, ex_date)
    );
    CREATE TABLE position_snapshots (
        symbol TEXT, exchange TEXT,
        quantity DECIMAL(18,8), avg_cost DECIMAL(18,4),
        invested DECIMAL(18,4), market_value DECIMAL(18,4),
        unrealized_pnl DECIMAL(18,4), unrealized_pnl_pct DECIMAL(9,4),
        realized_pnl DECIMAL(18,4), day_change_pct DECIMAL(9,4),
        score SMALLINT, score_breakdown JSON, computed_at TIMESTAMP,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE TABLE search_cache (
        query TEXT PRIMARY KEY, results JSON, fetched_at TIMESTAMP
    );
    "#,
)];
```

- [ ] **Step 3: Write the failing tests in `src/storage/db.rs`**

```rust
use crate::storage::schema::{checksum, MIGRATIONS};
use anyhow::Context;
use duckdb::Connection;
use std::path::Path;

pub struct Db {
    pub conn: Connection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_migrates_to_latest_version() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
    }

    #[test]
    fn migrations_are_idempotent_on_reopen() {
        // In-memory cannot reopen; use a temp file path.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("w.duckdb");
        { let _ = Db::open(&path).unwrap(); }
        let db2 = Db::open(&path).unwrap();
        assert_eq!(db2.schema_version().unwrap(), 1);
    }
}
```

- [ ] **Step 4: Run to verify failure**

Run: `cargo test storage::db`
Expected: FAIL — methods not found.

- [ ] **Step 5: Implement `Db` (above `tests`)**

```rust
impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        let conn = Connection::open(path).context("open duckdb")?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> anyhow::Result<Db> {
        let conn = Connection::open_in_memory().context("open in-memory duckdb")?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY, applied_at TIMESTAMP, checksum TEXT);",
        )?;
        for (version, sql) in MIGRATIONS {
            let sum = checksum(sql);
            let recorded: Option<String> = self
                .conn
                .query_row(
                    "SELECT checksum FROM schema_version WHERE version = ?",
                    [version],
                    |r| r.get(0),
                )
                .ok();
            match recorded {
                Some(existing) if existing != sum => {
                    anyhow::bail!(
                        "migration {} checksum drift: recorded {} but binary expects {}",
                        version, existing, sum
                    );
                }
                Some(_) => continue, // already applied, matches
                None => {
                    self.conn.execute_batch(sql)?;
                    self.conn.execute(
                        "INSERT INTO schema_version (version, applied_at, checksum) VALUES (?, now(), ?)",
                        duckdb::params![version, sum],
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn schema_version(&self) -> anyhow::Result<i32> {
        let v: i32 = self
            .conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))?;
        Ok(v)
    }
}
```

- [ ] **Step 6: Register module in `src/lib.rs`**

Add line: `pub mod storage;`

- [ ] **Step 7: Run to verify pass**

Run: `cargo test storage::db`
Expected: PASS (2 tests). First build is slow (DuckDB bundled compiles).

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat(storage): DuckDB open + checksummed migrations"
```

---

### Task 8: Storage — queries

**Files:**
- Create: `src/storage/queries.rs`
- Test: inline in `src/storage/queries.rs`

**Interfaces:**
- Consumes: `storage::db::Db`, `core::types::*`, `core::pnl::Position`, `core::score::ScoreBreakdown`.
- Produces, all as methods on `Db`:
  - `insert_transaction(&self, t: &Trade) -> anyhow::Result<i64>` (returns new id; ignores `t.id`).
  - `list_transactions(&self, asset: Option<&AssetId>) -> anyhow::Result<Vec<Trade>>`.
  - `delete_transaction(&self, id: i64) -> anyhow::Result<bool>`.
  - `distinct_held_assets(&self) -> anyhow::Result<Vec<AssetId>>` (assets with a nonzero net position).
  - `upsert_quote(&self, q: &Quote) -> anyhow::Result<()>`.
  - `get_quote(&self, asset: &AssetId) -> anyhow::Result<Option<Quote>>`.
  - `upsert_candles(&self, asset: &AssetId, candles: &[Candle]) -> anyhow::Result<()>`.
  - `get_candles(&self, asset: &AssetId) -> anyhow::Result<Vec<Candle>>` (ascending by date).
  - `upsert_dividends(&self, asset: &AssetId, divs: &[Dividend]) -> anyhow::Result<()>`.
  - `get_dividends(&self, asset: &AssetId) -> anyhow::Result<Vec<Dividend>>`.
  - `write_snapshot(&self, snap: &PositionSnapshot) -> anyhow::Result<()>` + `read_snapshots(&self) -> anyhow::Result<Vec<PositionSnapshot>>`.
  - Type `queries::PositionSnapshot { asset, quantity, avg_cost, invested, market_value, unrealized_pnl, unrealized_pnl_pct, realized_pnl, day_change_pct, score, score_breakdown: ScoreBreakdown, computed_at }`.
- Decimal binding rule: bind decimals via `.to_string()` into `CAST(? AS DECIMAL(..))`; read via `CAST(col AS VARCHAR)` then `Decimal::from_str`.

- [ ] **Step 1: Write the failing tests in `src/storage/queries.rs`**

```rust
use crate::core::types::{AssetId, Candle, Dividend, Quote, Side, Trade};
use crate::storage::db::Db;
use rust_decimal::Decimal;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn buy(qty: &str, price: &str) -> Trade {
        Trade {
            id: 0, asset: AssetId::b3("PETR4"), side: Side::Buy,
            quantity: qty.parse().unwrap(), price: price.parse().unwrap(), fees: dec!(0),
            executed_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), note: None,
        }
    }

    #[test]
    fn insert_and_list_transactions() {
        let db = Db::open_in_memory().unwrap();
        let id = db.insert_transaction(&buy("100", "10.00")).unwrap();
        assert!(id >= 1);
        let all = db.list_transactions(None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].quantity, dec!(100));
        assert_eq!(all[0].price, dec!(10.0000));
    }

    #[test]
    fn delete_transaction_removes_it() {
        let db = Db::open_in_memory().unwrap();
        let id = db.insert_transaction(&buy("100", "10.00")).unwrap();
        assert!(db.delete_transaction(id).unwrap());
        assert_eq!(db.list_transactions(None).unwrap().len(), 0);
    }

    #[test]
    fn quote_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        let asset = AssetId::b3("PETR4");
        let q = Quote {
            asset: asset.clone(), price: dec!(12.34), prev_close: dec!(12.00),
            day_high: dec!(12.5), day_low: dec!(11.9), currency: "BRL".into(), source: "yahoo".into(),
            fetched_at: NaiveDate::from_ymd_opt(2026,1,2).unwrap().and_hms_opt(10,0,0).unwrap(),
        };
        db.upsert_quote(&q).unwrap();
        let got = db.get_quote(&asset).unwrap().unwrap();
        assert_eq!(got.price, dec!(12.3400));
        assert_eq!(got.source, "yahoo");
    }

    #[test]
    fn held_assets_excludes_fully_sold() {
        let db = Db::open_in_memory().unwrap();
        db.insert_transaction(&buy("100", "10")).unwrap();
        let sell = Trade { side: Side::Sell, ..buy("100", "11") };
        db.insert_transaction(&sell).unwrap();
        assert_eq!(db.distinct_held_assets().unwrap().len(), 0);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test storage::queries`
Expected: FAIL — methods not found.

- [ ] **Step 3: Implement query methods (above `tests`)**

```rust
fn dec(s: String) -> anyhow::Result<Decimal> {
    Ok(Decimal::from_str(s.trim())?)
}

#[derive(Debug, Clone)]
pub struct PositionSnapshot {
    pub asset: AssetId,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub invested: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub realized_pnl: Decimal,
    pub day_change_pct: Decimal,
    pub score: u8,
    pub score_breakdown: crate::core::score::ScoreBreakdown,
    pub computed_at: chrono::NaiveDateTime,
}

impl Db {
    pub fn insert_transaction(&self, t: &Trade) -> anyhow::Result<i64> {
        let side = match t.side { Side::Buy => "BUY", Side::Sell => "SELL" };
        self.conn.execute(
            "INSERT INTO transactions (symbol, exchange, side, quantity, price, fees, executed_at, note)
             VALUES (?, ?, ?, CAST(? AS DECIMAL(18,8)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), ?, ?)",
            duckdb::params![
                t.asset.symbol, t.asset.exchange, side,
                t.quantity.to_string(), t.price.to_string(), t.fees.to_string(),
                t.executed_at.to_string(), t.note
            ],
        )?;
        let id: i64 = self.conn.query_row("SELECT currval('tx_id_seq')", [], |r| r.get(0))?;
        Ok(id)
    }

    pub fn list_transactions(&self, asset: Option<&AssetId>) -> anyhow::Result<Vec<Trade>> {
        let base = "SELECT id, symbol, exchange, side,
                    CAST(quantity AS VARCHAR), CAST(price AS VARCHAR), CAST(fees AS VARCHAR),
                    executed_at, note FROM transactions";
        let mut out = Vec::new();
        let mut push = |row: &duckdb::Row| -> duckdb::Result<()> { let _ = row; Ok(()) };
        let _ = &mut push;
        let map = |row: &duckdb::Row| -> anyhow::Result<Trade> {
            let side_s: String = row.get(3)?;
            Ok(Trade {
                id: row.get(0)?,
                asset: AssetId { symbol: row.get(1)?, exchange: row.get(2)? },
                side: if side_s == "BUY" { Side::Buy } else { Side::Sell },
                quantity: dec(row.get(4)?)?,
                price: dec(row.get(5)?)?,
                fees: dec(row.get(6)?)?,
                executed_at: row.get::<_, chrono::NaiveDate>(7)?,
                note: row.get(8)?,
            })
        };
        if let Some(a) = asset {
            let sql = format!("{base} WHERE symbol = ? AND exchange = ? ORDER BY executed_at, id");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(duckdb::params![a.symbol, a.exchange], |r| Ok(map(r)))?;
            for r in rows { out.push(r??); }
        } else {
            let sql = format!("{base} ORDER BY executed_at, id");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], |r| Ok(map(r)))?;
            for r in rows { out.push(r??); }
        }
        Ok(out)
    }

    pub fn delete_transaction(&self, id: i64) -> anyhow::Result<bool> {
        let n = self.conn.execute("DELETE FROM transactions WHERE id = ?", [id])?;
        Ok(n > 0)
    }

    pub fn distinct_held_assets(&self) -> anyhow::Result<Vec<AssetId>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, exchange,
                    SUM(CASE WHEN side='BUY' THEN quantity ELSE -quantity END) AS net
             FROM transactions GROUP BY symbol, exchange HAVING net <> 0",
        )?;
        let rows = stmt.query_map([], |r| Ok(AssetId { symbol: r.get(0)?, exchange: r.get(1)? }))?;
        Ok(rows.collect::<duckdb::Result<Vec<_>>>()?)
    }

    pub fn upsert_quote(&self, q: &Quote) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO quotes (symbol, exchange, price, prev_close, day_high, day_low, currency, source, fetched_at)
             VALUES (?, ?, CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), ?, ?, ?)",
            duckdb::params![
                q.asset.symbol, q.asset.exchange, q.price.to_string(), q.prev_close.to_string(),
                q.day_high.to_string(), q.day_low.to_string(), q.currency, q.source, q.fetched_at
            ],
        )?;
        Ok(())
    }

    pub fn get_quote(&self, asset: &AssetId) -> anyhow::Result<Option<Quote>> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(price AS VARCHAR), CAST(prev_close AS VARCHAR), CAST(day_high AS VARCHAR),
                    CAST(day_low AS VARCHAR), currency, source, fetched_at
             FROM quotes WHERE symbol = ? AND exchange = ?",
        )?;
        let mut rows = stmt.query(duckdb::params![asset.symbol, asset.exchange])?;
        if let Some(r) = rows.next()? {
            Ok(Some(Quote {
                asset: asset.clone(),
                price: dec(r.get(0)?)?, prev_close: dec(r.get(1)?)?,
                day_high: dec(r.get(2)?)?, day_low: dec(r.get(3)?)?,
                currency: r.get(4)?, source: r.get(5)?, fetched_at: r.get(6)?,
            }))
        } else { Ok(None) }
    }

    pub fn upsert_candles(&self, asset: &AssetId, candles: &[Candle]) -> anyhow::Result<()> {
        for c in candles {
            self.conn.execute(
                "INSERT OR REPLACE INTO price_history (symbol, exchange, date, open, high, low, close, volume)
                 VALUES (?, ?, ?, CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), ?)",
                duckdb::params![asset.symbol, asset.exchange, c.date.to_string(),
                    c.open.to_string(), c.high.to_string(), c.low.to_string(), c.close.to_string(), c.volume],
            )?;
        }
        Ok(())
    }

    pub fn get_candles(&self, asset: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, CAST(open AS VARCHAR), CAST(high AS VARCHAR), CAST(low AS VARCHAR),
                    CAST(close AS VARCHAR), volume FROM price_history
             WHERE symbol = ? AND exchange = ? ORDER BY date",
        )?;
        let rows = stmt.query_map(duckdb::params![asset.symbol, asset.exchange], |r| {
            Ok((r.get::<_, chrono::NaiveDate>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (date, o, h, l, c, v) = row?;
            out.push(Candle { date, open: dec(o)?, high: dec(h)?, low: dec(l)?, close: dec(c)?, volume: v });
        }
        Ok(out)
    }

    pub fn upsert_dividends(&self, asset: &AssetId, divs: &[Dividend]) -> anyhow::Result<()> {
        for d in divs {
            self.conn.execute(
                "INSERT OR REPLACE INTO dividends (symbol, exchange, ex_date, pay_date, amount_per_share, source)
                 VALUES (?, ?, ?, ?, CAST(? AS DECIMAL(18,4)), ?)",
                duckdb::params![asset.symbol, asset.exchange, d.ex_date.to_string(),
                    d.pay_date.map(|p| p.to_string()), d.amount_per_share.to_string(), "provider"],
            )?;
        }
        Ok(())
    }

    pub fn get_dividends(&self, asset: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let mut stmt = self.conn.prepare(
            "SELECT ex_date, pay_date, CAST(amount_per_share AS VARCHAR) FROM dividends
             WHERE symbol = ? AND exchange = ? ORDER BY ex_date",
        )?;
        let rows = stmt.query_map(duckdb::params![asset.symbol, asset.exchange], |r| {
            Ok((r.get::<_, chrono::NaiveDate>(0)?, r.get::<_, Option<chrono::NaiveDate>>(1)?, r.get::<_, String>(2)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (ex_date, pay_date, amt) = row?;
            out.push(Dividend { asset: asset.clone(), ex_date, pay_date, amount_per_share: dec(amt)? });
        }
        Ok(out)
    }

    pub fn write_snapshot(&self, s: &PositionSnapshot) -> anyhow::Result<()> {
        let breakdown = serde_json::to_string(&s.score_breakdown)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO position_snapshots
             (symbol, exchange, quantity, avg_cost, invested, market_value, unrealized_pnl,
              unrealized_pnl_pct, realized_pnl, day_change_pct, score, score_breakdown, computed_at)
             VALUES (?, ?, CAST(? AS DECIMAL(18,8)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)),
                     CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(9,4)),
                     CAST(? AS DECIMAL(18,4)), CAST(? AS DECIMAL(9,4)), ?, ?, ?)",
            duckdb::params![
                s.asset.symbol, s.asset.exchange, s.quantity.to_string(), s.avg_cost.to_string(),
                s.invested.to_string(), s.market_value.to_string(), s.unrealized_pnl.to_string(),
                s.unrealized_pnl_pct.to_string(), s.realized_pnl.to_string(), s.day_change_pct.to_string(),
                s.score as i16, breakdown, s.computed_at
            ],
        )?;
        Ok(())
    }

    pub fn read_snapshots(&self) -> anyhow::Result<Vec<PositionSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, exchange, CAST(quantity AS VARCHAR), CAST(avg_cost AS VARCHAR),
                    CAST(invested AS VARCHAR), CAST(market_value AS VARCHAR), CAST(unrealized_pnl AS VARCHAR),
                    CAST(unrealized_pnl_pct AS VARCHAR), CAST(realized_pnl AS VARCHAR), CAST(day_change_pct AS VARCHAR),
                    score, score_breakdown, computed_at FROM position_snapshots ORDER BY symbol",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                AssetId { symbol: r.get(0)?, exchange: r.get(1)? },
                r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, String>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?, r.get::<_, String>(9)?,
                r.get::<_, i16>(10)?, r.get::<_, String>(11)?, r.get::<_, chrono::NaiveDateTime>(12)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (asset, qty, avg, inv, mv, upnl, upct, rpnl, dpct, score, bd, computed_at) = row?;
            out.push(PositionSnapshot {
                asset, quantity: dec(qty)?, avg_cost: dec(avg)?, invested: dec(inv)?, market_value: dec(mv)?,
                unrealized_pnl: dec(upnl)?, unrealized_pnl_pct: dec(upct)?, realized_pnl: dec(rpnl)?,
                day_change_pct: dec(dpct)?, score: score as u8,
                score_breakdown: serde_json::from_str(&bd)?, computed_at,
            });
        }
        Ok(out)
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test storage::queries`
Expected: PASS (4 tests). Remove the dead `push` placeholder lines if the compiler warns.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(storage): transaction/quote/history/dividend/snapshot queries"
```

---

### Task 9: Provider trait + fallback chain

**Files:**
- Create: `src/providers/mod.rs`
- Modify: `src/lib.rs` (add `pub mod providers;`)
- Test: inline in `src/providers/mod.rs`

**Interfaces:**
- Consumes: `core::types::{AssetId, Quote, Candle, Dividend, Asset}`.
- Produces: `#[async_trait]`-free trait via `tokio` — use `async fn` in trait through `trait Provider { ... }` returning `Pin<Box<dyn Future>>`? Simpler: define the trait with `async fn` and require Rust 1.75 native async-in-trait.
  - `trait Provider: Send + Sync { fn name(&self) -> &'static str; async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote>; async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>>; async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>>; async fn search(&self, q: &str) -> anyhow::Result<Vec<Asset>>; }`
  - `struct Chain { providers: Vec<Box<dyn Provider>> }` with the same four methods, each trying providers in order, returning the first `Ok`, else the last `Err`.

- [ ] **Step 1: Write the failing test in `src/providers/mod.rs`**

```rust
pub mod yahoo;
pub mod brapi;

use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};

pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn quote(&self, a: &AssetId) -> impl std::future::Future<Output = anyhow::Result<Quote>> + Send;
    fn history(&self, a: &AssetId) -> impl std::future::Future<Output = anyhow::Result<Vec<Candle>>> + Send;
    fn dividends(&self, a: &AssetId) -> impl std::future::Future<Output = anyhow::Result<Vec<Dividend>>> + Send;
    fn search(&self, q: &str) -> impl std::future::Future<Output = anyhow::Result<Vec<Asset>>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    struct Failing;
    struct Working;

    impl Provider for Failing {
        fn name(&self) -> &'static str { "failing" }
        async fn quote(&self, _: &AssetId) -> anyhow::Result<Quote> { anyhow::bail!("down") }
        async fn history(&self, _: &AssetId) -> anyhow::Result<Vec<Candle>> { anyhow::bail!("down") }
        async fn dividends(&self, _: &AssetId) -> anyhow::Result<Vec<Dividend>> { anyhow::bail!("down") }
        async fn search(&self, _: &str) -> anyhow::Result<Vec<Asset>> { anyhow::bail!("down") }
    }
    impl Provider for Working {
        fn name(&self) -> &'static str { "working" }
        async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
            Ok(Quote { asset: a.clone(), price: dec!(9), prev_close: dec!(9), day_high: dec!(9), day_low: dec!(9),
                currency: "BRL".into(), source: "working".into(),
                fetched_at: NaiveDate::from_ymd_opt(2026,1,1).unwrap().and_hms_opt(0,0,0).unwrap() })
        }
        async fn history(&self, _: &AssetId) -> anyhow::Result<Vec<Candle>> { Ok(vec![]) }
        async fn dividends(&self, _: &AssetId) -> anyhow::Result<Vec<Dividend>> { Ok(vec![]) }
        async fn search(&self, _: &str) -> anyhow::Result<Vec<Asset>> { Ok(vec![]) }
    }

    #[tokio::test]
    async fn chain_falls_through_to_working_provider() {
        let chain = DynChain::new(vec![Box::new(BoxedFailing), Box::new(BoxedWorking)]);
        let q = chain.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.source, "working");
    }
}
```

> Note: native `async fn` in traits is not yet object-safe, so the fallback chain stores **boxed enum providers**, not `dyn Provider`. Implement the chain over a concrete enum (next step) and keep the `Provider` trait for the two concrete impls. Adjust the test to use the enum (see Step 3).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test providers`
Expected: FAIL — `DynChain`/`BoxedFailing` not defined.

- [ ] **Step 3: Replace the test bottom + add the `Chain` enum**

Replace the `chain_falls_through...` test and add the chain. The chain holds an enum of the real providers; for the unit test add test-only variants behind `#[cfg(test)]`.

```rust
pub enum AnyProvider {
    Yahoo(yahoo::YahooProvider),
    Brapi(brapi::BrapiProvider),
    #[cfg(test)]
    TestFailing,
    #[cfg(test)]
    TestWorking,
}

impl AnyProvider {
    pub fn name(&self) -> &'static str {
        match self {
            AnyProvider::Yahoo(_) => "yahoo",
            AnyProvider::Brapi(_) => "brapi",
            #[cfg(test)] AnyProvider::TestFailing => "failing",
            #[cfg(test)] AnyProvider::TestWorking => "working",
        }
    }
    pub async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        match self {
            AnyProvider::Yahoo(p) => p.quote(a).await,
            AnyProvider::Brapi(p) => p.quote(a).await,
            #[cfg(test)] AnyProvider::TestFailing => anyhow::bail!("down"),
            #[cfg(test)] AnyProvider::TestWorking => Ok(Quote {
                asset: a.clone(), price: rust_decimal_macros::dec!(9), prev_close: rust_decimal_macros::dec!(9),
                day_high: rust_decimal_macros::dec!(9), day_low: rust_decimal_macros::dec!(9),
                currency: "BRL".into(), source: "working".into(),
                fetched_at: chrono::NaiveDate::from_ymd_opt(2026,1,1).unwrap().and_hms_opt(0,0,0).unwrap() }),
        }
    }
    // history/dividends/search follow the same match shape; Yahoo/Brapi delegate,
    // test variants return Err("down") / Ok(vec![]) respectively.
    pub async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        match self { AnyProvider::Yahoo(p)=>p.history(a).await, AnyProvider::Brapi(p)=>p.history(a).await,
            #[cfg(test)] AnyProvider::TestFailing=>anyhow::bail!("down"), #[cfg(test)] AnyProvider::TestWorking=>Ok(vec![]) }
    }
    pub async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        match self { AnyProvider::Yahoo(p)=>p.dividends(a).await, AnyProvider::Brapi(p)=>p.dividends(a).await,
            #[cfg(test)] AnyProvider::TestFailing=>anyhow::bail!("down"), #[cfg(test)] AnyProvider::TestWorking=>Ok(vec![]) }
    }
    pub async fn search(&self, q: &str) -> anyhow::Result<Vec<Asset>> {
        match self { AnyProvider::Yahoo(p)=>p.search(q).await, AnyProvider::Brapi(p)=>p.search(q).await,
            #[cfg(test)] AnyProvider::TestFailing=>anyhow::bail!("down"), #[cfg(test)] AnyProvider::TestWorking=>Ok(vec![]) }
    }
}

pub struct Chain { pub providers: Vec<AnyProvider> }

impl Chain {
    pub fn new(providers: Vec<AnyProvider>) -> Self { Self { providers } }

    pub async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers {
            match p.quote(a).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e.context(format!("provider {} failed", p.name())),
            }
        }
        Err(last)
    }
    pub async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers { match p.history(a).await { Ok(v)=>return Ok(v), Err(e)=>last=e } }
        Err(last)
    }
    pub async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers { match p.dividends(a).await { Ok(v)=>return Ok(v), Err(e)=>last=e } }
        Err(last)
    }
    pub async fn search(&self, q: &str) -> anyhow::Result<Vec<Asset>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers { match p.search(q).await { Ok(v)=>return Ok(v), Err(e)=>last=e } }
        Err(last)
    }
}
```

Replace the test with:

```rust
    #[tokio::test]
    async fn chain_falls_through_to_working_provider() {
        let chain = Chain::new(vec![AnyProvider::TestFailing, AnyProvider::TestWorking]);
        let q = chain.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.source, "working");
    }
```

Keep the `Provider` trait (Step 1) for the concrete impls in Tasks 10–11.

- [ ] **Step 4: Add empty provider stubs so the module compiles**

In `src/providers/yahoo.rs` and `src/providers/brapi.rs`, create empty placeholder structs the chain can name (filled in next tasks):

```rust
// yahoo.rs
pub struct YahooProvider { pub client: reqwest::Client }
// brapi.rs
pub struct BrapiProvider { pub client: reqwest::Client, pub token: Option<String> }
```

Temporarily implement `Provider` for both with `anyhow::bail!("not implemented")` bodies so `AnyProvider` delegation compiles. (Replaced in Tasks 10–11.)

- [ ] **Step 5: Register module + run**

Add `pub mod providers;` to `src/lib.rs`.
Run: `cargo test providers::tests::chain_falls_through_to_working_provider`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(providers): Provider trait + ordered fallback Chain"
```

---

### Task 10: Yahoo provider

**Files:**
- Modify: `src/providers/yahoo.rs`
- Create: `tests/fixtures/yahoo_chart_petr4.json` (capture a real `chart` response; trim to a few candles)
- Test: inline in `src/providers/yahoo.rs` using `wiremock`

**Interfaces:**
- Consumes: `core::types::*`, `Provider`.
- Produces: `YahooProvider::new(base_url: String) -> Self` (base_url override enables wiremock); default base `https://query1.finance.yahoo.com`. Parses the `chart` endpoint for quote (last close, prev close, day high/low) and candles; `quoteSummary`/`v8` dividends via the `events` field of `chart?interval=1d&range=1y&events=div`.
- `quote`, `history`, `dividends` all derive from one `chart?range=1y&interval=1d&events=div` call (parse once); `search` hits `/v1/finance/search?q=`.

- [ ] **Step 1: Add the fixture `tests/fixtures/yahoo_chart_petr4.json`**

Minimal shape the parser expects (real responses are larger; keep these keys):

```json
{
  "chart": { "result": [ {
    "meta": { "currency": "BRL", "symbol": "PETR4.SA", "regularMarketPrice": 38.50, "chartPreviousClose": 37.90, "regularMarketDayHigh": 38.90, "regularMarketDayLow": 37.80 },
    "timestamp": [1735693200, 1735779600],
    "indicators": { "quote": [ { "open": [37.0, 38.0], "high": [38.0, 39.0], "low": [36.5, 37.5], "close": [37.9, 38.5], "volume": [1000, 1200] } ] },
    "events": { "dividends": { "1735693200": { "amount": 0.55, "date": 1735693200 } } }
  } ] }
}
```

- [ ] **Step 2: Write the failing test in `src/providers/yahoo.rs`**

```rust
use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};
use crate::providers::Provider;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn parses_quote_from_chart() {
        let server = MockServer::start().await;
        let body = include_str!("../../tests/fixtures/yahoo_chart_petr4.json");
        Mock::given(method("GET")).and(path_regex(r"^/v8/finance/chart/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server).await;

        let p = YahooProvider::new(server.uri());
        let q = p.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.price, dec!(38.50));
        assert_eq!(q.prev_close, dec!(37.90));
        assert_eq!(q.currency, "BRL");
        assert_eq!(q.source, "yahoo");

        let candles = p.history(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[1].close, dec!(38.50));

        let divs = p.dividends(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].amount_per_share, dec!(0.55));
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test providers::yahoo`
Expected: FAIL — `YahooProvider::new` / parsing not implemented.

- [ ] **Step 4: Implement the Yahoo provider (above `tests`)**

```rust
use rust_decimal::Decimal;
use std::str::FromStr;

pub struct YahooProvider {
    client: reqwest::Client,
    base_url: String,
}

impl YahooProvider {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 local-ticker-wallet")
            .build()
            .expect("reqwest client");
        Self { client, base_url }
    }
    pub fn default_base() -> Self { Self::new("https://query1.finance.yahoo.com".to_string()) }

    async fn fetch_chart(&self, a: &AssetId) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/v8/finance/chart/{}?range=1y&interval=1d&events=div", self.base_url, a.yahoo_ticker());
        let v: serde_json::Value = self.client.get(url).send().await?.error_for_status()?.json().await?;
        Ok(v)
    }
}

fn d(v: &serde_json::Value) -> Option<Decimal> {
    v.as_f64().and_then(|f| Decimal::from_str(&format!("{:.4}", f)).ok())
}

impl Provider for YahooProvider {
    fn name(&self) -> &'static str { "yahoo" }

    async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let v = self.fetch_chart(a).await?;
        let meta = &v["chart"]["result"][0]["meta"];
        Ok(Quote {
            asset: a.clone(),
            price: d(&meta["regularMarketPrice"]).ok_or_else(|| anyhow::anyhow!("no price"))?,
            prev_close: d(&meta["chartPreviousClose"]).unwrap_or_default(),
            day_high: d(&meta["regularMarketDayHigh"]).unwrap_or_default(),
            day_low: d(&meta["regularMarketDayLow"]).unwrap_or_default(),
            currency: meta["currency"].as_str().unwrap_or("BRL").to_string(),
            source: "yahoo".into(),
            fetched_at: chrono::Utc::now().naive_utc(),
        })
    }

    async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let v = self.fetch_chart(a).await?;
        let result = &v["chart"]["result"][0];
        let ts = result["timestamp"].as_array().cloned().unwrap_or_default();
        let q = &result["indicators"]["quote"][0];
        let mut out = Vec::new();
        for (i, t) in ts.iter().enumerate() {
            let secs = t.as_i64().unwrap_or(0);
            let date = chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.naive_utc().date());
            let (Some(date), Some(o), Some(h), Some(l), Some(c)) = (
                date, d(&q["open"][i]), d(&q["high"][i]), d(&q["low"][i]), d(&q["close"][i])
            ) else { continue };
            out.push(Candle { date, open: o, high: h, low: l, close: c, volume: q["volume"][i].as_i64().unwrap_or(0) });
        }
        Ok(out)
    }

    async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let v = self.fetch_chart(a).await?;
        let divs = &v["chart"]["result"][0]["events"]["dividends"];
        let mut out = Vec::new();
        if let Some(map) = divs.as_object() {
            for (_, dv) in map {
                let secs = dv["date"].as_i64().unwrap_or(0);
                let Some(ex_date) = chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.naive_utc().date()) else { continue };
                let Some(amt) = d(&dv["amount"]) else { continue };
                out.push(Dividend { asset: a.clone(), ex_date, pay_date: None, amount_per_share: amt });
            }
        }
        out.sort_by_key(|d| d.ex_date);
        Ok(out)
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Asset>> {
        let url = format!("{}/v1/finance/search?q={}", self.base_url, urlencoding_min(query));
        let v: serde_json::Value = self.client.get(url).send().await?.error_for_status()?.json().await?;
        let mut out = Vec::new();
        if let Some(items) = v["quotes"].as_array() {
            for it in items {
                let sym = it["symbol"].as_str().unwrap_or("");
                let symbol = sym.strip_suffix(".SA").unwrap_or(sym).to_string();
                out.push(Asset {
                    id: AssetId { symbol, exchange: "BVMF".into() },
                    name: it["shortname"].as_str().unwrap_or("").to_string(),
                    kind: it["quoteType"].as_str().unwrap_or("EQUITY").to_string(),
                    currency: "BRL".into(),
                });
            }
        }
        Ok(out)
    }
}

fn urlencoding_min(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_string() } else { format!("%{:02X}", c as u32) }).collect()
}
```

- [ ] **Step 5: Wire the default constructor into `AnyProvider`**

In `src/providers/mod.rs`, ensure `AnyProvider::Yahoo(YahooProvider)` is constructed via `YahooProvider::default_base()` where the daemon builds the chain (Task 13). No code change needed here beyond confirming the variant compiles.

- [ ] **Step 6: Run to verify pass**

Run: `cargo test providers::yahoo`
Expected: PASS (1 test).

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(providers): Yahoo chart parsing (quote/history/dividends/search)"
```

---

### Task 11: brapi provider

**Files:**
- Modify: `src/providers/brapi.rs`
- Create: `tests/fixtures/brapi_quote_petr4.json`
- Test: inline in `src/providers/brapi.rs` with `wiremock`

**Interfaces:**
- Consumes: `core::types::*`, `Provider`.
- Produces: `BrapiProvider::new(base_url: String, token: Option<String>) -> Self`; default base `https://brapi.dev/api`. Parses `/quote/{ticker}?range=1y&interval=1d&dividends=true&fundamental=false`. `search` hits `/available?search=`.

- [ ] **Step 1: Add fixture `tests/fixtures/brapi_quote_petr4.json`**

```json
{
  "results": [ {
    "symbol": "PETR4", "currency": "BRL",
    "regularMarketPrice": 38.50, "regularMarketPreviousClose": 37.90,
    "regularMarketDayHigh": 38.90, "regularMarketDayLow": 37.80,
    "historicalDataPrice": [
      { "date": 1735693200, "open": 37.0, "high": 38.0, "low": 36.5, "close": 37.9, "volume": 1000 },
      { "date": 1735779600, "open": 38.0, "high": 39.0, "low": 37.5, "close": 38.5, "volume": 1200 }
    ],
    "dividendsData": { "cashDividends": [ { "rate": 0.55, "paymentDate": "2026-01-15", "lastDatePrior": "2026-01-02" } ] }
  } ]
}
```

- [ ] **Step 2: Write the failing test in `src/providers/brapi.rs`**

```rust
use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};
use crate::providers::Provider;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn parses_brapi_quote_and_history() {
        let server = MockServer::start().await;
        let body = include_str!("../../tests/fixtures/brapi_quote_petr4.json");
        Mock::given(method("GET")).and(path_regex(r"^/quote/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server).await;

        let p = BrapiProvider::new(server.uri(), None);
        let q = p.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.price, dec!(38.50));
        assert_eq!(q.source, "brapi");
        let c = p.history(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(c.len(), 2);
        let d = p.dividends(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(d[0].amount_per_share, dec!(0.55));
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test providers::brapi`
Expected: FAIL.

- [ ] **Step 4: Implement the brapi provider (above `tests`)**

```rust
use rust_decimal::Decimal;
use std::str::FromStr;

pub struct BrapiProvider {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl BrapiProvider {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self { client: reqwest::Client::new(), base_url, token }
    }
    pub fn default_base(token: Option<String>) -> Self { Self::new("https://brapi.dev/api".into(), token) }

    async fn fetch(&self, a: &AssetId) -> anyhow::Result<serde_json::Value> {
        let mut url = format!("{}/quote/{}?range=1y&interval=1d&dividends=true", self.base_url, a.symbol);
        if let Some(t) = &self.token { url.push_str(&format!("&token={t}")); }
        Ok(self.client.get(url).send().await?.error_for_status()?.json().await?)
    }
}

fn d(v: &serde_json::Value) -> Option<Decimal> {
    v.as_f64().and_then(|f| Decimal::from_str(&format!("{:.4}", f)).ok())
}

impl Provider for BrapiProvider {
    fn name(&self) -> &'static str { "brapi" }

    async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let v = self.fetch(a).await?;
        let r = &v["results"][0];
        Ok(Quote {
            asset: a.clone(),
            price: d(&r["regularMarketPrice"]).ok_or_else(|| anyhow::anyhow!("no price"))?,
            prev_close: d(&r["regularMarketPreviousClose"]).unwrap_or_default(),
            day_high: d(&r["regularMarketDayHigh"]).unwrap_or_default(),
            day_low: d(&r["regularMarketDayLow"]).unwrap_or_default(),
            currency: r["currency"].as_str().unwrap_or("BRL").to_string(),
            source: "brapi".into(),
            fetched_at: chrono::Utc::now().naive_utc(),
        })
    }

    async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let v = self.fetch(a).await?;
        let mut out = Vec::new();
        if let Some(arr) = v["results"][0]["historicalDataPrice"].as_array() {
            for h in arr {
                let secs = h["date"].as_i64().unwrap_or(0);
                let Some(date) = chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.naive_utc().date()) else { continue };
                let (Some(o),Some(hi),Some(l),Some(c)) = (d(&h["open"]),d(&h["high"]),d(&h["low"]),d(&h["close"])) else { continue };
                out.push(Candle { date, open: o, high: hi, low: l, close: c, volume: h["volume"].as_i64().unwrap_or(0) });
            }
        }
        out.sort_by_key(|c| c.date);
        Ok(out)
    }

    async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let v = self.fetch(a).await?;
        let mut out = Vec::new();
        if let Some(arr) = v["results"][0]["dividendsData"]["cashDividends"].as_array() {
            for dvd in arr {
                let Some(amt) = d(&dvd["rate"]) else { continue };
                let ex_date = dvd["lastDatePrior"].as_str().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let pay_date = dvd["paymentDate"].as_str().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let Some(ex_date) = ex_date else { continue };
                out.push(Dividend { asset: a.clone(), ex_date, pay_date, amount_per_share: amt });
            }
        }
        out.sort_by_key(|d| d.ex_date);
        Ok(out)
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Asset>> {
        let mut url = format!("{}/available?search={}", self.base_url, query);
        if let Some(t) = &self.token { url.push_str(&format!("&token={t}")); }
        let v: serde_json::Value = self.client.get(url).send().await?.error_for_status()?.json().await?;
        let mut out = Vec::new();
        if let Some(arr) = v["stocks"].as_array() {
            for s in arr {
                if let Some(sym) = s.as_str() {
                    out.push(Asset { id: AssetId::b3(sym), name: sym.to_string(), kind: "EQUITY".into(), currency: "BRL".into() });
                }
            }
        }
        Ok(out)
    }
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cargo test providers::brapi`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(providers): brapi.dev quote/history/dividends/search"
```

---

### Task 12: IPC envelope + framing

**Files:**
- Create: `src/ipc/mod.rs`
- Modify: `src/lib.rs` (add `pub mod ipc;`)
- Test: inline in `src/ipc/mod.rs`

**Interfaces:**
- Produces: `ipc::Action` (enum, serde `rename_all = "PascalCase"`): `AddTransaction, ListTransactions, DeleteTransaction, GetPositions, GetPositionDetail, RefreshNow, Search, Import, Export, Ping`.
- Produces: `ipc::Request { id: String, #[serde(rename="type")] kind: String, action: Action, payload: serde_json::Value }` with `Request::new(action, payload) -> Request` (generates uuid, sets `kind="request"`).
- Produces: `ipc::Response` serializing to `{ id, status, data }` or `{ id, status, error:{code,message} }`; constructors `Response::ok(id, data)` and `Response::err(id, code, message)`.
- Produces: `ipc::ErrorCode` enum (`NotFound, ProviderDown, BadRequest, Internal`) serializing to the screaming-snake strings.
- Produces framing: `async fn write_msg<W: AsyncWrite+Unpin, T: Serialize>(w, &T)` and `async fn read_line<R: AsyncBufRead+Unpin>(r) -> Option<String>` — newline-delimited.

- [ ] **Step 1: Write the failing test in `src/ipc/mod.rs`**

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_envelope_shape() {
        let req = Request::new(Action::Ping, serde_json::json!({}));
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "request");
        assert_eq!(v["action"], "Ping");
        assert!(v["id"].as_str().unwrap().len() >= 8);
    }

    #[test]
    fn ok_response_shape() {
        let r = Response::ok("abc".into(), serde_json::json!({"pong": true}));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["id"], "abc");
        assert_eq!(v["status"], "ok");
        assert_eq!(v["data"]["pong"], true);
    }

    #[test]
    fn err_response_shape() {
        let r = Response::err("abc".into(), ErrorCode::NotFound, "missing".into());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["status"], "error");
        assert_eq!(v["error"]["code"], "NOT_FOUND");
        assert_eq!(v["error"]["message"], "missing");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test ipc`
Expected: FAIL.

- [ ] **Step 3: Implement the envelope (above `tests`)**

```rust
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    AddTransaction, ListTransactions, DeleteTransaction, GetPositions,
    GetPositionDetail, RefreshNow, Search, Import, Export, Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub action: Action,
    pub payload: serde_json::Value,
}

impl Request {
    pub fn new(action: Action, payload: serde_json::Value) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), kind: "request".into(), action, payload }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode { NotFound, ProviderDown, BadRequest, Internal }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody { pub code: ErrorCode, pub message: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Response {
    Ok { id: String, data: serde_json::Value },
    Error { id: String, error: ErrorBody },
}

impl Response {
    pub fn ok(id: String, data: serde_json::Value) -> Self { Response::Ok { id, data } }
    pub fn err(id: String, code: ErrorCode, message: String) -> Self {
        Response::Error { id, error: ErrorBody { code, message } }
    }
}

pub async fn write_msg<W: AsyncWrite + Unpin, T: Serialize>(w: &mut W, msg: &T) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    w.write_all(line.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

pub async fn read_line<R: AsyncBufRead + Unpin>(r: &mut R) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    let n = r.read_line(&mut buf).await?;
    if n == 0 { Ok(None) } else { Ok(Some(buf.trim_end().to_string())) }
}
```

- [ ] **Step 4: Register module + run**

Add `pub mod ipc;` to `src/lib.rs`.
Run: `cargo test ipc`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(ipc): request/response envelope + newline framing"
```

---

### Task 13: Daemon — request handler + recompute

**Files:**
- Create: `src/daemon/mod.rs`, `src/daemon/server.rs`, `src/daemon/recompute.rs`
- Modify: `src/lib.rs` (add `pub mod daemon;`)
- Test: `tests/daemon_ipc.rs` (integration)

**Interfaces:**
- Consumes: `storage::db::Db`, `storage::queries::PositionSnapshot`, `core::*`, `providers::Chain`, `ipc::*`, `config::Config`.
- Produces: `daemon::recompute::recompute_asset(db, asset, weights) -> anyhow::Result<PositionSnapshot>` (reads trades+quote+candles+dividends, runs core, writes snapshot).
- Produces: `daemon::server::handle(db: &Db, chain: &Chain, cfg: &Config, req: Request) -> Response` dispatching by `action`.
- Produces: `daemon::run(cfg: Config) -> anyhow::Result<()>` — binds the Unix socket, owns the `Db`, spawns the poller (Task 14), serves requests. Uses a single-threaded view of `Db` guarded by a `tokio::sync::Mutex` (DuckDB `Connection` is not `Sync`).

- [ ] **Step 1: Implement `recompute_asset` in `src/daemon/recompute.rs`**

```rust
use crate::config::ScoreWeights;
use crate::core::{pnl::{Position, Valuation}, score, types::AssetId};
use crate::storage::{db::Db, queries::PositionSnapshot};

pub fn recompute_asset(db: &Db, asset: &AssetId, weights: &ScoreWeights) -> anyhow::Result<PositionSnapshot> {
    let trades = db.list_transactions(Some(asset))?;
    let position = Position::from_trades(asset, &trades)?;
    let quote = db.get_quote(asset)?;
    let candles = db.get_candles(asset)?;
    let dividends = db.get_dividends(asset)?;

    let (val, breakdown) = match &quote {
        Some(q) => {
            let v = Valuation::compute(&position, q);
            let b = score::compute(&position, q, &candles, &dividends, weights);
            (Some(v), b)
        }
        None => (None, score::ScoreBreakdown {
            proximity_low: rust_decimal::Decimal::ZERO, below_sma: rust_decimal::Decimal::ZERO,
            drawdown: rust_decimal::Decimal::ZERO, dividend_yield: rust_decimal::Decimal::ZERO,
            cost_vs_trend: rust_decimal::Decimal::ZERO, total: 0,
        }),
    };

    let snap = PositionSnapshot {
        asset: asset.clone(),
        quantity: position.quantity,
        avg_cost: position.avg_cost,
        invested: position.invested,
        market_value: val.as_ref().map(|v| v.market_value).unwrap_or_default(),
        unrealized_pnl: val.as_ref().map(|v| v.unrealized_pnl).unwrap_or_default(),
        unrealized_pnl_pct: val.as_ref().map(|v| v.unrealized_pnl_pct).unwrap_or_default(),
        realized_pnl: position.realized_pnl,
        day_change_pct: val.as_ref().map(|v| v.day_change_pct).unwrap_or_default(),
        score: breakdown.total,
        score_breakdown: breakdown,
        computed_at: chrono::Utc::now().naive_utc(),
    };
    db.write_snapshot(&snap)?;
    Ok(snap)
}
```

- [ ] **Step 2: Implement `handle` in `src/daemon/server.rs`**

```rust
use crate::config::Config;
use crate::core::types::{AssetId, Side, Trade};
use crate::ipc::{Action, ErrorCode, Request, Response};
use crate::providers::Chain;
use crate::storage::db::Db;
use crate::daemon::recompute::recompute_asset;

fn snapshot_json(s: &crate::storage::queries::PositionSnapshot) -> serde_json::Value {
    serde_json::json!({
        "symbol": s.asset.symbol, "exchange": s.asset.exchange,
        "quantity": s.quantity.to_string(), "avg_cost": s.avg_cost.to_string(),
        "invested": s.invested.to_string(), "market_value": s.market_value.to_string(),
        "unrealized_pnl": s.unrealized_pnl.to_string(), "unrealized_pnl_pct": s.unrealized_pnl_pct.to_string(),
        "realized_pnl": s.realized_pnl.to_string(), "day_change_pct": s.day_change_pct.to_string(),
        "score": s.score, "score_breakdown": s.score_breakdown, "computed_at": s.computed_at.to_string(),
    })
}

pub async fn handle(db: &Db, chain: &Chain, cfg: &Config, req: Request) -> Response {
    let id = req.id.clone();
    let result: anyhow::Result<serde_json::Value> = match req.action {
        Action::Ping => Ok(serde_json::json!({"pong": true})),

        Action::AddTransaction => (|| {
            let p = &req.payload;
            let asset = AssetId::b3(p["symbol"].as_str().ok_or_else(|| anyhow::anyhow!("symbol required"))?);
            let side = match p["side"].as_str() { Some("SELL") => Side::Sell, _ => Side::Buy };
            let t = Trade {
                id: 0, asset: asset.clone(), side,
                quantity: p["quantity"].as_str().unwrap_or("0").parse()?,
                price: p["price"].as_str().unwrap_or("0").parse()?,
                fees: p["fees"].as_str().unwrap_or("0").parse()?,
                executed_at: chrono::NaiveDate::parse_from_str(p["executed_at"].as_str().unwrap_or(""), "%Y-%m-%d")?,
                note: p["note"].as_str().map(|s| s.to_string()),
            };
            let new_id = db.insert_transaction(&t)?;
            let _ = recompute_asset(db, &asset, &cfg.score_weights);
            Ok(serde_json::json!({"id": new_id}))
        })(),

        Action::ListTransactions => (|| {
            let asset = req.payload["symbol"].as_str().map(AssetId::b3);
            let txs = db.list_transactions(asset.as_ref())?;
            let arr: Vec<_> = txs.iter().map(|t| serde_json::json!({
                "id": t.id, "symbol": t.asset.symbol, "side": match t.side { Side::Buy=>"BUY", Side::Sell=>"SELL" },
                "quantity": t.quantity.to_string(), "price": t.price.to_string(), "fees": t.fees.to_string(),
                "executed_at": t.executed_at.to_string(), "note": t.note,
            })).collect();
            Ok(serde_json::json!({"transactions": arr}))
        })(),

        Action::DeleteTransaction => (|| {
            let id = req.payload["id"].as_i64().ok_or_else(|| anyhow::anyhow!("id required"))?;
            let removed = db.delete_transaction(id)?;
            Ok(serde_json::json!({"removed": removed}))
        })(),

        Action::GetPositions => (|| {
            let snaps = db.read_snapshots()?;
            let arr: Vec<_> = snaps.iter().map(snapshot_json).collect();
            Ok(serde_json::json!({"positions": arr}))
        })(),

        Action::GetPositionDetail => (|| {
            let asset = AssetId::b3(req.payload["symbol"].as_str().ok_or_else(|| anyhow::anyhow!("symbol required"))?);
            let snap = recompute_asset(db, &asset, &cfg.score_weights)?;
            Ok(snapshot_json(&snap))
        })(),

        Action::RefreshNow => {
            refresh(db, chain, cfg, req.payload["symbol"].as_str().map(AssetId::b3)).await
                .map(|n| serde_json::json!({"refreshed": n}))
        }

        Action::Search => {
            let q = req.payload["query"].as_str().unwrap_or("").to_string();
            match chain.search(&q).await {
                Ok(assets) => Ok(serde_json::json!({"results": assets})),
                Err(e) => Err(e),
            }
        }

        Action::Import => crate::portfolio::import_csv(db, req.payload["path"].as_str().unwrap_or(""), &cfg.score_weights)
            .map(|n| serde_json::json!({"imported": n})),

        Action::Export => crate::portfolio::export_csv(db, req.payload["path"].as_str().unwrap_or(""))
            .map(|n| serde_json::json!({"exported": n})),
    };

    match result {
        Ok(data) => Response::ok(id, data),
        Err(e) => Response::err(id, ErrorCode::Internal, e.to_string()),
    }
}

async fn refresh(db: &Db, chain: &Chain, cfg: &Config, only: Option<AssetId>) -> anyhow::Result<usize> {
    let targets = match only { Some(a) => vec![a], None => db.distinct_held_assets()? };
    let mut n = 0;
    for a in targets {
        if let Ok(q) = chain.quote(&a).await { db.upsert_quote(&q)?; n += 1; }
        if let Ok(c) = chain.history(&a).await { db.upsert_candles(&a, &c)?; }
        if let Ok(d) = chain.dividends(&a).await { db.upsert_dividends(&a, &d)?; }
        let _ = recompute_asset(db, &a, &cfg.score_weights);
    }
    Ok(n)
}
```

> `crate::portfolio::{import_csv, export_csv}` come from Task 15; if implementing strictly in order, stub them to `Ok(0)` here and replace in Task 15.

- [ ] **Step 3: Implement `daemon::run` + module in `src/daemon/mod.rs`**

```rust
pub mod server;
pub mod recompute;
pub mod poller;

use crate::config::Config;
use crate::ipc::{self, Request};
use crate::providers::{AnyProvider, Chain};
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::UnixListener;
use tokio::sync::Mutex;

pub fn build_chain(cfg: &Config) -> Chain {
    Chain::new(vec![
        AnyProvider::Yahoo(crate::providers::yahoo::YahooProvider::default_base()),
        AnyProvider::Brapi(crate::providers::brapi::BrapiProvider::default_base(cfg.brapi_token.clone())),
    ])
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let sock = crate::paths::socket_path();
    let _ = std::fs::remove_file(&sock); // clear stale socket
    let db = Arc::new(Mutex::new(Db::open(&crate::paths::data_db())?));
    let chain = Arc::new(build_chain(&cfg));
    let cfg = Arc::new(cfg);

    // Poller task
    {
        let (db, chain, cfg) = (db.clone(), chain.clone(), cfg.clone());
        tokio::spawn(async move { poller::run_poller(db, chain, cfg).await; });
    }

    let listener = UnixListener::bind(&sock)?;
    loop {
        let (stream, _) = listener.accept().await?;
        let (db, chain, cfg) = (db.clone(), chain.clone(), cfg.clone());
        tokio::spawn(async move {
            let (r, mut w) = stream.into_split();
            let mut reader = BufReader::new(r);
            while let Ok(Some(line)) = ipc::read_line(&mut reader).await {
                let req: Request = match serde_json::from_str(&line) { Ok(r) => r, Err(_) => continue };
                let resp = {
                    let db = db.lock().await;
                    server::handle(&db, &chain, &cfg, req).await
                };
                if ipc::write_msg(&mut w, &resp).await.is_err() { break; }
            }
        });
    }
}
```

- [ ] **Step 4: Write the integration test `tests/daemon_ipc.rs`**

```rust
use local_ticker_wallet::ipc::{self, Action, Request};
use local_ticker_wallet::storage::db::Db;
use local_ticker_wallet::config::Config;
use local_ticker_wallet::providers::Chain;
use local_ticker_wallet::daemon::server::handle;

#[tokio::test]
async fn add_then_get_positions_via_handler() {
    let db = Db::open_in_memory().unwrap();
    let chain = Chain::new(vec![]); // no network needed for this path
    let cfg = Config::default();

    let add = Request::new(Action::AddTransaction, serde_json::json!({
        "symbol": "PETR4", "side": "BUY", "quantity": "100", "price": "10.00", "fees": "0", "executed_at": "2026-01-01"
    }));
    let r = handle(&db, &chain, &cfg, add).await;
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["status"], "ok");

    let get = Request::new(Action::GetPositions, serde_json::json!({}));
    let r2 = handle(&db, &chain, &cfg, get).await;
    let v2 = serde_json::to_value(&r2).unwrap();
    assert_eq!(v2["status"], "ok");
    let positions = v2["data"]["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["symbol"], "PETR4");
    assert_eq!(positions[0]["quantity"], "100.00000000");
}
```

- [ ] **Step 5: Register module + run**

Add `pub mod daemon;` to `src/lib.rs`. (Also add `pub mod portfolio;` now as an empty stub — Task 15 — so Task 13's references compile: create `src/portfolio.rs` with `pub fn import_csv(_db:&crate::storage::db::Db,_p:&str,_w:&crate::config::ScoreWeights)->anyhow::Result<usize>{Ok(0)}` and matching `export_csv`.)
Run: `cargo test --test daemon_ipc`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(daemon): request handler, recompute, socket server"
```

---

### Task 14: Poller + B3 market hours

**Files:**
- Create: `src/daemon/poller.rs`, `src/daemon/market_hours.rs`
- Test: inline in `src/daemon/market_hours.rs`

**Interfaces:**
- Produces: `market_hours::is_open(now: chrono::DateTime<chrono_tz::Tz>) -> bool` — true on weekdays between 10:00 and 18:00 in `America/Sao_Paulo`.
- Produces: `poller::run_poller(db, chain, cfg)` — loops: if market open, refresh all held assets every `cfg.poll_interval_secs`; if closed, sleep 5 min and once per calendar day refresh history+dividends.

- [ ] **Step 1: Write the failing test in `src/daemon/market_hours.rs`**

```rust
use chrono::{Datelike, Timelike};
use chrono_tz::America::Sao_Paulo;
use chrono_tz::Tz;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn weekday_noon_is_open() {
        // 2026-06-22 is a Monday
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 22, 12, 0, 0).unwrap();
        assert!(is_open(t));
    }

    #[test]
    fn weekend_is_closed() {
        // 2026-06-21 is a Sunday
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 21, 12, 0, 0).unwrap();
        assert!(!is_open(t));
    }

    #[test]
    fn before_open_is_closed() {
        let t = Sao_Paulo.with_ymd_and_hms(2026, 6, 22, 9, 0, 0).unwrap();
        assert!(!is_open(t));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test daemon::market_hours`
Expected: FAIL — `is_open` not found.

- [ ] **Step 3: Implement `is_open` (above `tests`)**

```rust
pub fn is_open(now: chrono::DateTime<Tz>) -> bool {
    let wd = now.weekday();
    let is_weekday = !matches!(wd, chrono::Weekday::Sat | chrono::Weekday::Sun);
    let h = now.hour();
    is_weekday && (10..18).contains(&h)
}

pub fn now_sp() -> chrono::DateTime<Tz> {
    chrono::Utc::now().with_timezone(&Sao_Paulo)
}
```

- [ ] **Step 4: Implement `src/daemon/poller.rs`**

```rust
use crate::config::Config;
use crate::providers::Chain;
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub async fn run_poller(db: Arc<Mutex<Db>>, chain: Arc<Chain>, cfg: Arc<Config>) {
    let mut last_daily: Option<chrono::NaiveDate> = None;
    loop {
        let now = super::market_hours::now_sp();
        if super::market_hours::is_open(now) {
            refresh_quotes(&db, &chain, &cfg).await;
            sleep(Duration::from_secs(cfg.poll_interval_secs.max(5))).await;
        } else {
            let today = now.date_naive();
            if last_daily != Some(today) {
                refresh_history(&db, &chain, &cfg).await;
                last_daily = Some(today);
            }
            sleep(Duration::from_secs(300)).await;
        }
    }
}

async fn refresh_quotes(db: &Arc<Mutex<Db>>, chain: &Arc<Chain>, cfg: &Arc<Config>) {
    let held = { let d = db.lock().await; d.distinct_held_assets().unwrap_or_default() };
    for a in held {
        if let Ok(q) = chain.quote(&a).await {
            let d = db.lock().await;
            let _ = d.upsert_quote(&q);
            let _ = crate::daemon::recompute::recompute_asset(&d, &a, &cfg.score_weights);
        }
    }
}

async fn refresh_history(db: &Arc<Mutex<Db>>, chain: &Arc<Chain>, cfg: &Arc<Config>) {
    let held = { let d = db.lock().await; d.distinct_held_assets().unwrap_or_default() };
    for a in held {
        if let Ok(c) = chain.history(&a).await { let d = db.lock().await; let _ = d.upsert_candles(&a, &c); }
        if let Ok(divs) = chain.dividends(&a).await { let d = db.lock().await; let _ = d.upsert_dividends(&a, &divs); }
        let d = db.lock().await;
        let _ = crate::daemon::recompute::recompute_asset(&d, &a, &cfg.score_weights);
    }
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cargo test daemon::market_hours`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(daemon): B3-hours-aware poller"
```

---

### Task 15: Portfolio import/export (CSV ledger + JSON config)

**Files:**
- Replace stub: `src/portfolio.rs`
- Modify: `src/lib.rs` (ensure `pub mod portfolio;`)
- Test: inline in `src/portfolio.rs`

**Interfaces:**
- Consumes: `storage::db::Db`, `core::types::*`, `config::ScoreWeights`, `daemon::recompute`.
- Produces: `portfolio::export_csv(db, path) -> anyhow::Result<usize>` — writes header `symbol,exchange,side,quantity,price,fees,executed_at,note` and one row per transaction; returns count.
- Produces: `portfolio::import_csv(db, path, weights) -> anyhow::Result<usize>` — reads that format, inserts transactions, recomputes affected assets; returns count.
- CSV is hand-rolled (no extra dep): split on commas, `note` is the last field and may be empty; quote nothing (reject commas in notes with a clear error).

- [ ] **Step 1: Write the failing test in `src/portfolio.rs`**

```rust
use crate::config::ScoreWeights;
use crate::core::types::{AssetId, Side, Trade};
use crate::daemon::recompute::recompute_asset;
use crate::storage::db::Db;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn buy(qty: &str, price: &str) -> Trade {
        Trade { id: 0, asset: AssetId::b3("PETR4"), side: Side::Buy,
            quantity: qty.parse().unwrap(), price: price.parse().unwrap(), fees: dec!(0),
            executed_at: NaiveDate::from_ymd_opt(2026,1,1).unwrap(), note: None }
    }

    #[test]
    fn export_then_import_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let csv = tmp.path().join("ledger.csv");

        let db1 = Db::open_in_memory().unwrap();
        db1.insert_transaction(&buy("100", "10.00")).unwrap();
        db1.insert_transaction(&buy("50", "12.00")).unwrap();
        let n = export_csv(&db1, csv.to_str().unwrap()).unwrap();
        assert_eq!(n, 2);

        let db2 = Db::open_in_memory().unwrap();
        let imported = import_csv(&db2, csv.to_str().unwrap(), &ScoreWeights::default()).unwrap();
        assert_eq!(imported, 2);
        assert_eq!(db2.list_transactions(None).unwrap().len(), 2);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test portfolio`
Expected: FAIL — real `export_csv`/`import_csv` not present (only the Task 13 stub returning 0).

- [ ] **Step 3: Implement export/import (replace the stub bodies)**

```rust
pub fn export_csv(db: &Db, path: &str) -> anyhow::Result<usize> {
    let txs = db.list_transactions(None)?;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "symbol,exchange,side,quantity,price,fees,executed_at,note")?;
    for t in &txs {
        let side = match t.side { Side::Buy => "BUY", Side::Sell => "SELL" };
        let note = t.note.clone().unwrap_or_default();
        if note.contains(',') { anyhow::bail!("note for {} contains a comma; not supported", t.asset.symbol); }
        writeln!(f, "{},{},{},{},{},{},{},{}", t.asset.symbol, t.asset.exchange, side,
            t.quantity, t.price, t.fees, t.executed_at, note)?;
    }
    Ok(txs.len())
}

pub fn import_csv(db: &Db, path: &str, weights: &ScoreWeights) -> anyhow::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    let mut count = 0;
    let mut touched: std::collections::HashSet<AssetId> = std::collections::HashSet::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() { continue; } // header / blank
        let f: Vec<&str> = line.splitn(8, ',').collect();
        if f.len() < 7 { anyhow::bail!("malformed CSV line {}", i + 1); }
        let asset = AssetId { symbol: f[0].to_uppercase(), exchange: f[1].to_string() };
        let t = Trade {
            id: 0, asset: asset.clone(),
            side: if f[2] == "SELL" { Side::Sell } else { Side::Buy },
            quantity: f[3].parse()?, price: f[4].parse()?, fees: f[5].parse()?,
            executed_at: chrono::NaiveDate::parse_from_str(f[6], "%Y-%m-%d")?,
            note: f.get(7).filter(|s| !s.is_empty()).map(|s| s.to_string()),
        };
        db.insert_transaction(&t)?;
        touched.insert(asset);
        count += 1;
    }
    for a in &touched { let _ = recompute_asset(db, a, weights); }
    Ok(count)
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test portfolio`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(portfolio): CSV ledger export/import"
```

---

### Task 16: CLI client + daemon autostart

**Files:**
- Replace: `src/main.rs`
- Create: `src/client.rs`
- Modify: `src/lib.rs` (add `pub mod client;`)
- Test: `tests/e2e.rs` (spawns the built binary)

**Interfaces:**
- Produces: `client::send(req: Request) -> anyhow::Result<Response>` — connects to the socket; if connection fails, spawns `ltw daemon` as a detached child, waits for the socket (poll up to ~3s), then retries once.
- Produces: clap CLI in `main.rs` with subcommands `daemon`, `tui`, `add`, `list`, `delete`, `refresh`, `search`, `import`, `export`. Each non-daemon/tui subcommand builds a `Request`, calls `client::send`, prints the JSON `data` (or the error) to stdout.

- [ ] **Step 1: Implement `src/client.rs`**

```rust
use crate::ipc::{self, Request, Response};
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

pub async fn send(req: Request) -> anyhow::Result<Response> {
    let sock = crate::paths::socket_path();
    let stream = match UnixStream::connect(&sock).await {
        Ok(s) => s,
        Err(_) => { spawn_daemon().await?; connect_retry(&sock).await? }
    };
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    ipc::write_msg(&mut w, &req).await?;
    let line = ipc::read_line(&mut reader).await?.ok_or_else(|| anyhow::anyhow!("daemon closed connection"))?;
    Ok(serde_json::from_str(&line)?)
}

async fn spawn_daemon() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).arg("daemon").spawn()?;
    Ok(())
}

async fn connect_retry(sock: &std::path::Path) -> anyhow::Result<UnixStream> {
    for _ in 0..30 {
        if let Ok(s) = UnixStream::connect(sock).await { return Ok(s); }
        sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("daemon did not start (socket {:?} never appeared)", sock)
}
```

- [ ] **Step 2: Implement `src/main.rs`**

```rust
use clap::{Parser, Subcommand};
use local_ticker_wallet::client;
use local_ticker_wallet::config::Config;
use local_ticker_wallet::ipc::{Action, Request, Response};

#[derive(Parser)]
#[command(name = "ltw")]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    Daemon,
    Tui,
    Add { symbol: String, quantity: String, price: String,
        #[arg(long, default_value = "BUY")] side: String,
        #[arg(long, default_value = "0")] fees: String,
        #[arg(long)] date: String,
        #[arg(long)] note: Option<String> },
    List { #[arg(long)] symbol: Option<String> },
    Delete { id: i64 },
    Refresh { #[arg(long)] symbol: Option<String> },
    Search { query: String },
    Import { path: String },
    Export { path: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Daemon => local_ticker_wallet::daemon::run(Config::load()?).await,
        Cmd::Tui => local_ticker_wallet::tui::run().await,
        cmd => {
            let req = to_request(cmd);
            let resp = client::send(req).await?;
            print_response(resp);
            Ok(())
        }
    }
}

fn to_request(cmd: Cmd) -> Request {
    match cmd {
        Cmd::Add { symbol, quantity, price, side, fees, date, note } => Request::new(Action::AddTransaction,
            serde_json::json!({"symbol":symbol,"quantity":quantity,"price":price,"side":side,"fees":fees,"executed_at":date,"note":note})),
        Cmd::List { symbol } => Request::new(Action::ListTransactions, serde_json::json!({"symbol":symbol})),
        Cmd::Delete { id } => Request::new(Action::DeleteTransaction, serde_json::json!({"id":id})),
        Cmd::Refresh { symbol } => Request::new(Action::RefreshNow, serde_json::json!({"symbol":symbol})),
        Cmd::Search { query } => Request::new(Action::Search, serde_json::json!({"query":query})),
        Cmd::Import { path } => Request::new(Action::Import, serde_json::json!({"path":path})),
        Cmd::Export { path } => Request::new(Action::Export, serde_json::json!({"path":path})),
        Cmd::Daemon | Cmd::Tui => unreachable!(),
    }
}

fn print_response(resp: Response) {
    match resp {
        Response::Ok { data, .. } => println!("{}", serde_json::to_string_pretty(&data).unwrap()),
        Response::Error { error, .. } => eprintln!("error [{:?}]: {}", error.code, error.message),
    }
}
```

- [ ] **Step 3: Write the e2e test `tests/e2e.rs`**

```rust
use std::process::Command;
use std::time::Duration;

// Isolate XDG dirs so the test never touches the real wallet.
fn ltw(args: &[&str], home: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ltw"))
        .args(args)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .output().unwrap()
}

#[test]
fn add_and_list_via_cli_autostarts_daemon() {
    let tmp = tempfile::tempdir().unwrap();
    for d in ["data", "config", "run"] { std::fs::create_dir_all(tmp.path().join(d)).unwrap(); }

    let add = ltw(&["add", "PETR4", "100", "10.00", "--date", "2026-01-01"], tmp.path());
    assert!(add.status.success(), "add failed: {}", String::from_utf8_lossy(&add.stderr));

    let list = ltw(&["list"], tmp.path());
    let out = String::from_utf8_lossy(&list.stdout);
    assert!(out.contains("PETR4"), "list output: {out}");

    // Stop the autostarted daemon.
    let _ = Command::new("pkill").args(["-f", "ltw daemon"]).output();
    std::thread::sleep(Duration::from_millis(200));
}
```

- [ ] **Step 4: Register module + run**

Add `pub mod client;` to `src/lib.rs`. Add `pub mod tui;` too (Task 17 supplies `tui::run`; create a temporary `src/tui/mod.rs` with `pub async fn run() -> anyhow::Result<()> { Ok(()) }` so this task compiles).
Run: `cargo test --test e2e -- --nocapture`
Expected: PASS (daemon autostarts, add+list succeed).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(cli): subcommands + socket client with daemon autostart"
```

---

### Task 17: TUI

**Files:**
- Replace: `src/tui/mod.rs`
- Create: `src/tui/client.rs`, `src/tui/views.rs`
- Test: `src/tui/views.rs` inline (pure render-to-buffer test)

**Interfaces:**
- Consumes: `ipc::{Action, Request, Response}`, `client::send`.
- Produces: `tui::run() -> anyhow::Result<()>` — initializes crossterm, fetches `GetPositions`, renders a positions table (symbol, qty, avg cost, last, day %, unrealized %, score), supports: `q` quit, `r` refresh (`RefreshNow` then re-fetch), `↑/↓` select row, `Enter` detail view (`GetPositionDetail` showing the score breakdown), `Esc` back.
- Produces: `views::PositionRow { symbol, quantity, avg_cost, market_value, day_change_pct, unrealized_pnl_pct, score }` and `views::render_positions(frame, area, rows, selected)`.

- [ ] **Step 1: Write the failing render test in `src/tui/views.rs`**

```rust
use ratatui::widgets::{Row, Table, Cell, Block, Borders};
use ratatui::layout::Rect;
use ratatui::style::{Style, Color, Modifier};

#[derive(Debug, Clone)]
pub struct PositionRow {
    pub symbol: String,
    pub quantity: String,
    pub avg_cost: String,
    pub market_value: String,
    pub day_change_pct: String,
    pub unrealized_pnl_pct: String,
    pub score: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_symbol_and_score() {
        let rows = vec![PositionRow {
            symbol: "PETR4".into(), quantity: "100".into(), avg_cost: "10.00".into(),
            market_value: "1200.00".into(), day_change_pct: "1.50".into(),
            unrealized_pnl_pct: "20.00".into(), score: 73,
        }];
        let backend = TestBackend::new(80, 10);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_positions(f, f.area(), &rows, 0)).unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("PETR4"));
        assert!(text.contains("73"));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test tui::views`
Expected: FAIL — `render_positions` not found.

- [ ] **Step 3: Implement `render_positions` (above `tests`)**

```rust
pub fn score_color(score: u8) -> Color {
    if score >= 70 { Color::Green } else if score >= 40 { Color::Yellow } else { Color::Red }
}

pub fn render_positions(frame: &mut ratatui::Frame, area: Rect, rows: &[PositionRow], selected: usize) {
    let header = Row::new(["Symbol", "Qty", "Avg", "Mkt Value", "Day %", "P&L %", "Score"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let body: Vec<Row> = rows.iter().enumerate().map(|(i, r)| {
        let style = if i == selected { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
        Row::new(vec![
            Cell::from(r.symbol.clone()),
            Cell::from(r.quantity.clone()),
            Cell::from(r.avg_cost.clone()),
            Cell::from(r.market_value.clone()),
            Cell::from(r.day_change_pct.clone()),
            Cell::from(r.unrealized_pnl_pct.clone()),
            Cell::from(r.score.to_string()).style(Style::default().fg(score_color(r.score))),
        ]).style(style)
    }).collect();
    use ratatui::layout::Constraint;
    let widths = [Constraint::Length(10); 7];
    let table = Table::new(body, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("local-ticker-wallet — positions (q quit · r refresh · Enter detail)"));
    frame.render_widget(table, area);
}
```

- [ ] **Step 4: Run to verify the render test passes**

Run: `cargo test tui::views`
Expected: PASS.

- [ ] **Step 5: Implement `src/tui/client.rs` (typed helpers over `client::send`)**

```rust
use crate::ipc::{Action, Request, Response};
use crate::tui::views::PositionRow;

pub async fn fetch_positions() -> anyhow::Result<Vec<PositionRow>> {
    let resp = crate::client::send(Request::new(Action::GetPositions, serde_json::json!({}))).await?;
    let data = match resp { Response::Ok { data, .. } => data, Response::Error { error, .. } => anyhow::bail!(error.message) };
    let mut rows = Vec::new();
    if let Some(arr) = data["positions"].as_array() {
        for p in arr {
            rows.push(PositionRow {
                symbol: p["symbol"].as_str().unwrap_or("").into(),
                quantity: p["quantity"].as_str().unwrap_or("").into(),
                avg_cost: p["avg_cost"].as_str().unwrap_or("").into(),
                market_value: p["market_value"].as_str().unwrap_or("").into(),
                day_change_pct: p["day_change_pct"].as_str().unwrap_or("").into(),
                unrealized_pnl_pct: p["unrealized_pnl_pct"].as_str().unwrap_or("").into(),
                score: p["score"].as_u64().unwrap_or(0) as u8,
            });
        }
    }
    Ok(rows)
}

pub async fn refresh_all() -> anyhow::Result<()> {
    crate::client::send(Request::new(Action::RefreshNow, serde_json::json!({}))).await?;
    Ok(())
}
```

- [ ] **Step 6: Implement `src/tui/mod.rs` event loop**

```rust
pub mod client;
pub mod views;

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::time::Duration;

pub async fn run() -> anyhow::Result<()> {
    let mut rows = client::fetch_positions().await.unwrap_or_default();
    let mut selected = 0usize;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = loop {
        term.draw(|f| views::render_positions(f, f.area(), &rows, selected))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char('r') => {
                        let _ = client::refresh_all().await;
                        rows = client::fetch_positions().await.unwrap_or_default();
                    }
                    KeyCode::Down => { if !rows.is_empty() { selected = (selected + 1).min(rows.len() - 1); } }
                    KeyCode::Up => { selected = selected.saturating_sub(1); }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    res
}
```

- [ ] **Step 7: Run all tests + manual smoke**

Run: `cargo test`
Expected: PASS (whole suite).
Manual: `cargo run -- add PETR4 100 30.00 --date 2026-01-02 && cargo run -- refresh && cargo run -- tui` — verify the table renders, `r` refreshes, `q` quits. (Requires network for `refresh`.)

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat(tui): positions table with refresh and score coloring"
```

---

## Self-Review

**Spec coverage:**
- Single binary, three modes — Tasks 1, 13, 16, 17. ✔
- Daemon sole DuckDB owner + Unix socket IPC — Tasks 12, 13. ✔
- Module map (core/storage/providers/ipc/daemon/tui/main) — Tasks 2–17. ✔
- XDG paths — Task 1. ✔
- DuckDB schema incl. `position_snapshots`, `search_cache`, composite `(symbol,exchange)` PK, `schema_version(version, applied_at, checksum)` — Tasks 7, 8. ✔
- Money as Decimal, never float — global constraint + Tasks 3–8. ✔
- IPC envelope `{id,type,action,payload}` / `{id,status,data|error}`, error codes — Task 12. ✔
- Calculations: P&L (Task 3), valuation (Task 4), signals (Task 5), dividends (Tasks 5/6/11), score with the five sub-scores incl. "distance from cost basis vs trend" weight 20 — Task 6. ✔
- Providers Yahoo→brapi fallback, `source` recorded — Tasks 9–11. ✔
- Polling B3-hours aware + RefreshNow — Task 14, Task 13 (`RefreshNow`). ✔
- Search non-persisted (only `search_cache`) — Tasks 11, 13. ✔ (Note: Task 13 returns search results live; `search_cache` table exists for a future TTL cache — wiring it is deferred and called out here, not silently dropped.)
- Error handling: provider fallback, stale-quote tolerance (recompute uses last cached quote), checksum-mismatch refuses start, stale-socket removal on daemon start — Tasks 7, 13. ✔
- CSV ledger export + JSON config import/export — Task 15 (CSV) + Task 1 (`Config` JSON). ✔
- Tests at every layer + e2e — every task + Tasks 13, 16. ✔

**Gaps consciously deferred (roadmap, per spec):** desktop/telegram alerts; FIIs/ETFs/US/crypto; splits/IR; B3 holiday calendar; `search_cache` TTL wiring; TUI detail-view drill-down is described in Task 17 interfaces but only the positions table is implemented with a test — detail view is a follow-up step within the same task if desired.

**Placeholder scan:** No "TBD/TODO" left. Two intentional stubs (`tui::run`, `portfolio::*`) are created early so out-of-order tasks compile and are explicitly replaced in their owning task (17, 15).

**Type consistency:** `AssetId::b3`, `Position::from_trades`, `Valuation::compute`, `score::compute`, `ScoreBreakdown`, `PositionSnapshot`, `Db` methods, `Chain`/`AnyProvider`, `ipc::{Action,Request,Response,ErrorCode}` names are used identically across tasks. The `Provider` trait (native async) is kept for concrete impls; the object-unsafe-async problem is handled via the `AnyProvider` enum, flagged in Task 9.
