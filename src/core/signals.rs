use crate::core::types::Candle;
use rust_decimal::Decimal;

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
    if candles.len() < days || days == 0 { return None; }
    let start = candles[candles.len() - days].close;
    let latest = candles[candles.len() - 1].close;
    if start.is_zero() { return Some(Decimal::ZERO); }
    Some((latest - start) / start * Decimal::from(100))
}

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
