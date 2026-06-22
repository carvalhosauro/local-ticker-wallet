use crate::ipc::{Action, Request, Response};
use crate::tui::views::{DetailData, PositionRow};

pub async fn fetch_positions() -> anyhow::Result<Vec<PositionRow>> {
    let resp = crate::client::send(Request::new(Action::GetPositions, serde_json::json!({}))).await?;
    let data = match resp {
        Response::Ok { data, .. } => data,
        Response::Error { error, .. } => anyhow::bail!(error.message),
    };
    let mut rows = Vec::new();
    if let Some(arr) = data["positions"].as_array() {
        for p in arr {
            rows.push(PositionRow {
                symbol: p["symbol"].as_str().unwrap_or("").into(),
                quantity: p["quantity"].as_str().unwrap_or("").into(),
                avg_cost: p["avg_cost"].as_str().unwrap_or("").into(),
                market_value: p["market_value"].as_str().unwrap_or("").into(),
                day_change_pct: p["day_change_pct"].as_str().unwrap_or("").into(),
                unrealized_pnl_pct: p["unrealized_pnl_pct"].as_str().unwrap_or("").into(),
                score: p["score"].as_u64().unwrap_or(0) as u8,
            });
        }
    }
    Ok(rows)
}

pub async fn refresh_all() -> anyhow::Result<()> {
    crate::client::send(Request::new(Action::RefreshNow, serde_json::json!({}))).await?;
    Ok(())
}

/// Fetches the full detail (including the score sub-scores) for a single symbol
/// via `Action::GetPositionDetail`.
pub async fn fetch_detail(symbol: &str) -> anyhow::Result<DetailData> {
    let resp = crate::client::send(Request::new(
        Action::GetPositionDetail,
        serde_json::json!({ "symbol": symbol }),
    ))
    .await?;
    let data = match resp {
        Response::Ok { data, .. } => data,
        Response::Error { error, .. } => anyhow::bail!(error.message),
    };
    // `score_breakdown` sub-scores are Decimals serialized as JSON strings
    // (rust_decimal "serde-with-str"); `total` is a u8 number.
    let bd = &data["score_breakdown"];
    Ok(DetailData {
        symbol: data["symbol"].as_str().unwrap_or("").into(),
        quantity: data["quantity"].as_str().unwrap_or("").into(),
        avg_cost: data["avg_cost"].as_str().unwrap_or("").into(),
        market_value: data["market_value"].as_str().unwrap_or("").into(),
        unrealized_pnl: data["unrealized_pnl"].as_str().unwrap_or("").into(),
        unrealized_pnl_pct: data["unrealized_pnl_pct"].as_str().unwrap_or("").into(),
        day_change_pct: data["day_change_pct"].as_str().unwrap_or("").into(),
        proximity_low: bd["proximity_low"].as_str().unwrap_or("").into(),
        below_sma: bd["below_sma"].as_str().unwrap_or("").into(),
        drawdown: bd["drawdown"].as_str().unwrap_or("").into(),
        dividend_yield: bd["dividend_yield"].as_str().unwrap_or("").into(),
        cost_vs_trend: bd["cost_vs_trend"].as_str().unwrap_or("").into(),
        total: bd["total"].as_u64().unwrap_or(0) as u8,
    })
}
