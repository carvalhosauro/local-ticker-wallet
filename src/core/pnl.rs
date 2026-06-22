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

    /// Reject a new trade if appending it would produce an invalid position.
    pub fn validate_append(
        asset: &AssetId,
        existing: &[Trade],
        new_trade: &Trade,
    ) -> Result<(), PnlError> {
        let next_id = existing.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let mut t = new_trade.clone();
        t.id = next_id;
        let mut trades = existing.to_vec();
        trades.push(t);
        Position::from_trades(asset, &trades)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Valuation {
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub day_change_pct: Decimal,
}

impl Valuation {
    pub fn compute(position: &Position, quote: &crate::core::types::Quote) -> Valuation {
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

    #[test]
    fn validate_append_rejects_oversell() {
        let asset = AssetId::b3("PETR4");
        let existing = vec![trade(1, Side::Buy, "10", "10.00", "0", 1)];
        let sell = trade(0, Side::Sell, "20", "11.00", "0", 2);
        assert!(matches!(
            Position::validate_append(&asset, &existing, &sell),
            Err(PnlError::Oversell { .. })
        ));
    }

    #[test]
    fn validate_append_accepts_valid_sell() {
        let asset = AssetId::b3("PETR4");
        let existing = vec![trade(1, Side::Buy, "10", "10.00", "0", 1)];
        let sell = trade(0, Side::Sell, "5", "11.00", "0", 2);
        Position::validate_append(&asset, &existing, &sell).unwrap();
    }

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
}
