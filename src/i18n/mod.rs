mod en;
mod pt_br;

pub use en::BUNDLE as EN;
pub use pt_br::BUNDLE as PT_BR;

use crate::config::Locale;

/// All user-visible strings for the TUI.
#[derive(Debug, Clone, Copy)]
pub struct Bundle {
    pub app_title: &'static str,
    // Navigation tabs
    pub nav_portfolio: &'static str,
    pub nav_search: &'static str,
    pub nav_ledger: &'static str,
    // Portfolio table
    pub col_symbol: &'static str,
    pub col_qty: &'static str,
    pub col_avg: &'static str,
    pub col_mkt_value: &'static str,
    pub col_day_pct: &'static str,
    pub col_pnl_pct: &'static str,
    pub col_score: &'static str,
    pub portfolio_footer: &'static str,
    // Detail
    pub detail_title: &'static str,
    pub label_symbol: &'static str,
    pub label_quantity: &'static str,
    pub label_avg_cost: &'static str,
    pub label_market_value: &'static str,
    pub label_unrealized_pnl: &'static str,
    pub label_day_pct: &'static str,
    pub score_breakdown: &'static str,
    pub score_proximity_low: &'static str,
    pub score_below_sma: &'static str,
    pub score_drawdown: &'static str,
    pub score_dividend_yield: &'static str,
    pub score_cost_vs_trend: &'static str,
    pub score_total: &'static str,
    pub detail_footer: &'static str,
    // Search
    pub search_title: &'static str,
    pub search_placeholder: &'static str,
    pub search_col_symbol: &'static str,
    pub search_col_name: &'static str,
    pub search_col_kind: &'static str,
    pub search_col_currency: &'static str,
    pub search_footer: &'static str,
    pub search_no_results: &'static str,
    pub search_in_portfolio: &'static str,
    pub search_not_in_portfolio: &'static str,
    // Ledger
    pub ledger_title: &'static str,
    pub ledger_col_id: &'static str,
    pub ledger_col_symbol: &'static str,
    pub ledger_col_side: &'static str,
    pub ledger_col_qty: &'static str,
    pub ledger_col_price: &'static str,
    pub ledger_col_date: &'static str,
    pub ledger_footer: &'static str,
    pub ledger_empty: &'static str,
    // Side labels
    pub side_buy: &'static str,
    pub side_sell: &'static str,
    // Errors / status
    pub err_fetch_positions: &'static str,
    pub err_fetch_detail: &'static str,
    pub err_refresh: &'static str,
    pub err_search: &'static str,
    pub err_fetch_ledger: &'static str,
}

pub fn bundle(locale: Locale) -> &'static Bundle {
    match locale {
        Locale::PtBr => &PT_BR,
        Locale::En => &EN,
    }
}
