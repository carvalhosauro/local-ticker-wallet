use crate::config::ScoreWeights;
use crate::core::{
    pnl::{Position, Valuation},
    score,
    types::{AssetId, Trade},
};
use crate::storage::{db::Db, queries::PositionSnapshot};
use rust_decimal::Decimal;

/// Rejects invalid trades before they are persisted (oversell, non-positive price/qty).
pub fn validate_trade(db: &Db, trade: &Trade) -> anyhow::Result<()> {
    if trade.price <= Decimal::ZERO {
        anyhow::bail!("price must be greater than zero");
    }
    if trade.quantity <= Decimal::ZERO {
        anyhow::bail!("quantity must be greater than zero");
    }
    let mut trades = db.list_transactions(Some(&trade.asset))?;
    trades.push(trade.clone());
    Position::from_trades(&trade.asset, &trades)?;
    Ok(())
}

pub fn recompute_asset(
    db: &Db,
    asset: &AssetId,
    weights: &ScoreWeights,
) -> anyhow::Result<PositionSnapshot> {
    let trades = db.list_transactions(Some(asset))?;
    if trades.is_empty() {
        db.delete_snapshot(asset)?;
        // Return an empty snapshot shape for callers that ignore the result.
        return Ok(PositionSnapshot {
            asset: asset.clone(),
            quantity: rust_decimal::Decimal::ZERO,
            avg_cost: rust_decimal::Decimal::ZERO,
            invested: rust_decimal::Decimal::ZERO,
            market_value: rust_decimal::Decimal::ZERO,
            unrealized_pnl: rust_decimal::Decimal::ZERO,
            unrealized_pnl_pct: rust_decimal::Decimal::ZERO,
            realized_pnl: rust_decimal::Decimal::ZERO,
            day_change_pct: rust_decimal::Decimal::ZERO,
            score: 0,
            score_breakdown: score::ScoreBreakdown {
                proximity_low: rust_decimal::Decimal::ZERO,
                below_sma: rust_decimal::Decimal::ZERO,
                drawdown: rust_decimal::Decimal::ZERO,
                dividend_yield: rust_decimal::Decimal::ZERO,
                cost_vs_trend: rust_decimal::Decimal::ZERO,
                total: 0,
            },
            computed_at: chrono::Utc::now().naive_utc(),
        });
    }
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
        None => (
            None,
            score::ScoreBreakdown {
                proximity_low: rust_decimal::Decimal::ZERO,
                below_sma: rust_decimal::Decimal::ZERO,
                drawdown: rust_decimal::Decimal::ZERO,
                dividend_yield: rust_decimal::Decimal::ZERO,
                cost_vs_trend: rust_decimal::Decimal::ZERO,
                total: 0,
            },
        ),
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
    if position.quantity.is_zero() {
        db.delete_snapshot(asset)?;
    } else {
        db.write_snapshot(&snap)?;
    }
    Ok(snap)
}
