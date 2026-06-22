use crate::core::types::Side;
use ratatui::style::Color;
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct PositionRow {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub day_change_pct: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub score: u8,
}

#[derive(Debug, Clone)]
pub struct DetailData {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub day_change_pct: Decimal,
    pub proximity_low: Decimal,
    pub below_sma: Decimal,
    pub drawdown: Decimal,
    pub dividend_yield: Decimal,
    pub cost_vs_trend: Decimal,
    pub total: u8,
}

#[derive(Debug, Clone)]
pub struct SearchPreview {
    pub symbol: String,
    pub name: String,
    pub kind: String,
    pub currency: String,
    pub in_portfolio: bool,
    pub price: rust_decimal::Decimal,
    pub day_change_pct: rust_decimal::Decimal,
}

#[derive(Debug, Clone)]
pub struct SearchResultRow {
    pub symbol: String,
    pub name: String,
    pub kind: String,
    pub currency: String,
    pub in_portfolio: bool,
}

#[derive(Debug, Clone)]
pub struct LedgerRow {
    pub id: i64,
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub executed_at: String,
}

pub fn score_color(score: u8) -> Color {
    if score >= 70 {
        Color::Green
    } else if score >= 40 {
        Color::Yellow
    } else {
        Color::Red
    }
}
