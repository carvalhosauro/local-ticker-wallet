//! CSV import/export of the portfolio ledger.

use crate::config::ScoreWeights;
use crate::core::types::{AssetId, Side, Trade};
use crate::daemon::recompute::recompute_asset;
use crate::storage::db::Db;
use std::io::Write;

/// Export the current portfolio transactions to a CSV file.
///
/// Writes header `symbol,exchange,side,quantity,price,fees,executed_at,note`
/// followed by one row per transaction. Returns the number of rows written.
pub fn export_csv(db: &Db, path: &str) -> anyhow::Result<usize> {
    let txs = db.list_transactions(None)?;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "symbol,exchange,side,quantity,price,fees,executed_at,note")?;
    for t in &txs {
        let side = match t.side {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
        };
        let note = t.note.clone().unwrap_or_default();
        if note.contains(',') {
            anyhow::bail!(
                "note for {} contains a comma; not supported",
                t.asset.symbol
            );
        }
        writeln!(
            f,
            "{},{},{},{},{},{},{},{}",
            t.asset.symbol,
            t.asset.exchange,
            side,
            t.quantity,
            t.price,
            t.fees,
            t.executed_at,
            note
        )?;
    }
    Ok(txs.len())
}

/// Import transactions from a CSV file, recomputing snapshots as it goes.
///
/// Reads the format written by [`export_csv`], inserts each transaction into
/// `db`, and calls `recompute_asset` for every asset touched. Returns the
/// number of rows imported.
pub fn import_csv(db: &Db, path: &str, weights: &ScoreWeights) -> anyhow::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    let mut count = 0;
    let mut touched: std::collections::HashSet<AssetId> = std::collections::HashSet::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // header / blank
        }
        let f: Vec<&str> = line.splitn(8, ',').collect();
        if f.len() < 7 {
            anyhow::bail!("malformed CSV line {}", i + 1);
        }
        let asset = AssetId {
            symbol: f[0].to_uppercase(),
            exchange: f[1].to_string(),
        };
        let t = Trade {
            id: 0,
            asset: asset.clone(),
            side: if f[2] == "SELL" {
                Side::Sell
            } else {
                Side::Buy
            },
            quantity: f[3].parse()?,
            price: f[4].parse()?,
            fees: f[5].parse()?,
            executed_at: chrono::NaiveDate::parse_from_str(f[6], "%Y-%m-%d")?,
            note: f.get(7).filter(|s| !s.is_empty()).map(|s| s.to_string()),
        };
        db.insert_transaction(&t)?;
        touched.insert(asset);
        count += 1;
    }
    for a in &touched {
        if let Err(e) = recompute_asset(db, a, weights) {
            eprintln!("warn: import_csv: recompute {} failed: {e}", a.symbol);
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn buy(qty: &str, price: &str) -> Trade {
        Trade {
            id: 0,
            asset: AssetId::b3("PETR4"),
            side: Side::Buy,
            quantity: qty.parse().unwrap(),
            price: price.parse().unwrap(),
            fees: dec!(0),
            executed_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            note: None,
        }
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
