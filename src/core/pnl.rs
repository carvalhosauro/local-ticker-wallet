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
