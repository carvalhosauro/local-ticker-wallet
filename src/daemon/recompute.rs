use crate::config::ScoreWeights;
use crate::core::{
    pnl::{Position, Valuation},
    score,
    types::AssetId,
};
use crate::storage::{db::Db, queries::PositionSnapshot};

pub fn recompute_asset(
    db: &Db,
    asset: &AssetId,
    weights: &ScoreWeights,
) -> anyhow::Result<PositionSnapshot> {
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
    db.write_snapshot(&snap)?;
    Ok(snap)
}
