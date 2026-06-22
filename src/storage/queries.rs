use crate::core::types::{AssetId, Candle, Dividend, Quote, Side, Trade};
use crate::storage::db::Db;
use rust_decimal::Decimal;
use std::str::FromStr;

fn parse_dec(s: String) -> anyhow::Result<Decimal> {
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                t.asset.symbol, t.asset.exchange, side,
                t.quantity.to_string(), t.price.to_string(), t.fees.to_string(),
                t.executed_at, t.note
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_transactions(&self, asset: Option<&AssetId>) -> anyhow::Result<Vec<Trade>> {
        let base = "SELECT id, symbol, exchange, side, quantity, price, fees, executed_at, note FROM transactions";
        let map_row = |r: &rusqlite::Row| -> rusqlite::Result<(i64, String, String, String, String, String, String, chrono::NaiveDate, Option<String>)> {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
        };
        let mut out = Vec::new();
        let rows: Vec<_> = if let Some(a) = asset {
            let sql = format!("{base} WHERE symbol = ?1 AND exchange = ?2 ORDER BY executed_at, id");
            let mut stmt = self.conn.prepare(&sql)?;
            let x = stmt.query_map(rusqlite::params![a.symbol, a.exchange], map_row)?.collect::<rusqlite::Result<Vec<_>>>()?; x
        } else {
            let sql = format!("{base} ORDER BY executed_at, id");
            let mut stmt = self.conn.prepare(&sql)?;
            let x = stmt.query_map([], map_row)?.collect::<rusqlite::Result<Vec<_>>>()?; x
        };
        for (id, sym, exch, side_s, qty, price, fees, executed_at, note) in rows {
            out.push(Trade {
                id,
                asset: AssetId { symbol: sym, exchange: exch },
                side: if side_s == "BUY" { Side::Buy } else { Side::Sell },
                quantity: parse_dec(qty)?,
                price: parse_dec(price)?,
                fees: parse_dec(fees)?,
                executed_at,
                note,
            });
        }
        Ok(out)
    }

    pub fn delete_transaction(&self, id: i64) -> anyhow::Result<bool> {
        let n = self.conn.execute("DELETE FROM transactions WHERE id = ?1", rusqlite::params![id])?;
        Ok(n > 0)
    }

    pub fn distinct_held_assets(&self) -> anyhow::Result<Vec<AssetId>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, exchange,
                    SUM(CASE WHEN side='BUY' THEN CAST(quantity AS REAL) ELSE -CAST(quantity AS REAL) END) AS net
             FROM transactions GROUP BY symbol, exchange HAVING net <> 0",
        )?;
        let rows = stmt.query_map([], |r| Ok(AssetId { symbol: r.get(0)?, exchange: r.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_quote(&self, q: &Quote) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO quotes (symbol, exchange, price, prev_close, day_high, day_low, currency, source, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                q.asset.symbol, q.asset.exchange, q.price.to_string(), q.prev_close.to_string(),
                q.day_high.to_string(), q.day_low.to_string(), q.currency, q.source, q.fetched_at
            ],
        )?;
        Ok(())
    }

    pub fn get_quote(&self, asset: &AssetId) -> anyhow::Result<Option<Quote>> {
        use rusqlite::OptionalExtension;
        let result = self.conn.query_row(
            "SELECT price, prev_close, day_high, day_low, currency, source, fetched_at
             FROM quotes WHERE symbol = ?1 AND exchange = ?2",
            rusqlite::params![asset.symbol, asset.exchange],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                    r.get::<_, chrono::NaiveDateTime>(6)?)),
        ).optional()?;
        match result {
            Some((price, prev_close, day_high, day_low, currency, source, fetched_at)) => Ok(Some(Quote {
                asset: asset.clone(),
                price: parse_dec(price)?, prev_close: parse_dec(prev_close)?,
                day_high: parse_dec(day_high)?, day_low: parse_dec(day_low)?,
                currency, source, fetched_at,
            })),
            None => Ok(None),
        }
    }

    pub fn upsert_candles(&self, asset: &AssetId, candles: &[Candle]) -> anyhow::Result<()> {
        for c in candles {
            self.conn.execute(
                "INSERT OR REPLACE INTO price_history (symbol, exchange, date, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![asset.symbol, asset.exchange, c.date,
                    c.open.to_string(), c.high.to_string(), c.low.to_string(), c.close.to_string(), c.volume],
            )?;
        }
        Ok(())
    }

    pub fn get_candles(&self, asset: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, open, high, low, close, volume FROM price_history
             WHERE symbol = ?1 AND exchange = ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(rusqlite::params![asset.symbol, asset.exchange], |r| {
            Ok((r.get::<_, chrono::NaiveDate>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (date, o, h, l, c, v) = row?;
            out.push(Candle { date, open: parse_dec(o)?, high: parse_dec(h)?, low: parse_dec(l)?, close: parse_dec(c)?, volume: v });
        }
        Ok(out)
    }

    pub fn upsert_dividends(&self, asset: &AssetId, divs: &[Dividend]) -> anyhow::Result<()> {
        for d in divs {
            self.conn.execute(
                "INSERT OR REPLACE INTO dividends (symbol, exchange, ex_date, pay_date, amount_per_share, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![asset.symbol, asset.exchange, d.ex_date, d.pay_date,
                    d.amount_per_share.to_string(), "provider"],
            )?;
        }
        Ok(())
    }

    pub fn get_dividends(&self, asset: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let mut stmt = self.conn.prepare(
            "SELECT ex_date, pay_date, amount_per_share FROM dividends
             WHERE symbol = ?1 AND exchange = ?2 ORDER BY ex_date",
        )?;
        let rows = stmt.query_map(rusqlite::params![asset.symbol, asset.exchange], |r| {
            Ok((r.get::<_, chrono::NaiveDate>(0)?, r.get::<_, Option<chrono::NaiveDate>>(1)?, r.get::<_, String>(2)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (ex_date, pay_date, amt) = row?;
            out.push(Dividend { asset: asset.clone(), ex_date, pay_date, amount_per_share: parse_dec(amt)? });
        }
        Ok(out)
    }

    pub fn write_snapshot(&self, s: &PositionSnapshot) -> anyhow::Result<()> {
        let breakdown = serde_json::to_string(&s.score_breakdown)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO position_snapshots
             (symbol, exchange, quantity, avg_cost, invested, market_value, unrealized_pnl,
              unrealized_pnl_pct, realized_pnl, day_change_pct, score, score_breakdown, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                s.asset.symbol, s.asset.exchange, s.quantity.to_string(), s.avg_cost.to_string(),
                s.invested.to_string(), s.market_value.to_string(), s.unrealized_pnl.to_string(),
                s.unrealized_pnl_pct.to_string(), s.realized_pnl.to_string(), s.day_change_pct.to_string(),
                s.score as i64, breakdown, s.computed_at
            ],
        )?;
        Ok(())
    }

    pub fn read_snapshots(&self) -> anyhow::Result<Vec<PositionSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, exchange, quantity, avg_cost, invested, market_value, unrealized_pnl,
                    unrealized_pnl_pct, realized_pnl, day_change_pct, score, score_breakdown, computed_at
             FROM position_snapshots ORDER BY symbol",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                AssetId { symbol: r.get(0)?, exchange: r.get(1)? },
                r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, String>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?, r.get::<_, String>(9)?,
                r.get::<_, i64>(10)?, r.get::<_, String>(11)?, r.get::<_, chrono::NaiveDateTime>(12)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (asset, qty, avg, inv, mv, upnl, upct, rpnl, dpct, score, bd, computed_at) = row?;
            out.push(PositionSnapshot {
                asset, quantity: parse_dec(qty)?, avg_cost: parse_dec(avg)?, invested: parse_dec(inv)?,
                market_value: parse_dec(mv)?, unrealized_pnl: parse_dec(upnl)?, unrealized_pnl_pct: parse_dec(upct)?,
                realized_pnl: parse_dec(rpnl)?, day_change_pct: parse_dec(dpct)?, score: score as u8,
                score_breakdown: serde_json::from_str(&bd)?, computed_at,
            });
        }
        Ok(out)
    }
}

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
