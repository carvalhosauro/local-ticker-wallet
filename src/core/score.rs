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
