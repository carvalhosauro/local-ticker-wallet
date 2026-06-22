use crate::core::format::parse_decimal;
use crate::core::types::Side;
use crate::ipc::{Action, Request, Response};
use crate::tui::models::{DetailData, LedgerRow, PositionRow, SearchResultRow};

async fn send(action: Action, payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let resp = crate::client::send(Request::new(action, payload)).await?;
    match resp {
        Response::Ok { data, .. } => Ok(data),
        Response::Error { error, .. } => anyhow::bail!(error.message),
    }
}

pub async fn fetch_positions() -> anyhow::Result<Vec<PositionRow>> {
    let data = send(Action::GetPositions, serde_json::json!({})).await?;
    let mut rows = Vec::new();
    if let Some(arr) = data["positions"].as_array() {
        for p in arr {
            rows.push(PositionRow {
                symbol: p["symbol"].as_str().unwrap_or("").into(),
                quantity: parse_decimal(p["quantity"].as_str().unwrap_or("0")).unwrap_or_default(),
                avg_cost: parse_decimal(p["avg_cost"].as_str().unwrap_or("0")).unwrap_or_default(),
                market_value: parse_decimal(p["market_value"].as_str().unwrap_or("0"))
                    .unwrap_or_default(),
                day_change_pct: parse_decimal(p["day_change_pct"].as_str().unwrap_or("0"))
                    .unwrap_or_default(),
                unrealized_pnl_pct: parse_decimal(p["unrealized_pnl_pct"].as_str().unwrap_or("0"))
                    .unwrap_or_default(),
                score: p["score"].as_u64().unwrap_or(0) as u8,
            });
        }
    }
    Ok(rows)
}

pub async fn refresh_all() -> anyhow::Result<()> {
    send(Action::RefreshNow, serde_json::json!({})).await?;
    Ok(())
}

pub async fn refresh_symbol(symbol: &str) -> anyhow::Result<()> {
    send(
        Action::RefreshNow,
        serde_json::json!({ "symbol": symbol }),
    )
    .await?;
    Ok(())
}

pub async fn fetch_detail(symbol: &str) -> anyhow::Result<DetailData> {
    let data = send(
        Action::GetPositionDetail,
        serde_json::json!({ "symbol": symbol }),
    )
    .await?;
    let bd = &data["score_breakdown"];
    Ok(DetailData {
        symbol: data["symbol"].as_str().unwrap_or("").into(),
        quantity: parse_decimal(data["quantity"].as_str().unwrap_or("0")).unwrap_or_default(),
        avg_cost: parse_decimal(data["avg_cost"].as_str().unwrap_or("0")).unwrap_or_default(),
        market_value: parse_decimal(data["market_value"].as_str().unwrap_or("0")).unwrap_or_default(),
        unrealized_pnl: parse_decimal(data["unrealized_pnl"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        unrealized_pnl_pct: parse_decimal(data["unrealized_pnl_pct"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        day_change_pct: parse_decimal(data["day_change_pct"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        proximity_low: parse_decimal(bd["proximity_low"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        below_sma: parse_decimal(bd["below_sma"].as_str().unwrap_or("0")).unwrap_or_default(),
        drawdown: parse_decimal(bd["drawdown"].as_str().unwrap_or("0")).unwrap_or_default(),
        dividend_yield: parse_decimal(bd["dividend_yield"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        cost_vs_trend: parse_decimal(bd["cost_vs_trend"].as_str().unwrap_or("0"))
            .unwrap_or_default(),
        total: bd["total"].as_u64().unwrap_or(0) as u8,
    })
}

pub async fn search_assets(query: &str) -> anyhow::Result<Vec<SearchResultRow>> {
    let data = send(Action::Search, serde_json::json!({ "query": query })).await?;
    let mut rows = Vec::new();
    if let Some(arr) = data["results"].as_array() {
        for r in arr {
            rows.push(SearchResultRow {
                symbol: r["id"]["symbol"].as_str().unwrap_or("").into(),
                name: r["name"].as_str().unwrap_or("").into(),
                kind: r["kind"].as_str().unwrap_or("").into(),
                currency: r["currency"].as_str().unwrap_or("").into(),
                in_portfolio: false,
            });
        }
    }
    Ok(rows)
}

pub async fn fetch_ledger() -> anyhow::Result<Vec<LedgerRow>> {
    let data = send(Action::ListTransactions, serde_json::json!({})).await?;
    let mut rows = Vec::new();
    if let Some(arr) = data["transactions"].as_array() {
        for t in arr {
            let side = match t["side"].as_str().unwrap_or("BUY") {
                "SELL" => Side::Sell,
                _ => Side::Buy,
            };
            rows.push(LedgerRow {
                id: t["id"].as_i64().unwrap_or(0),
                symbol: t["asset"]["symbol"]
                    .as_str()
                    .or_else(|| t["symbol"].as_str())
                    .unwrap_or("")
                    .into(),
                side,
                quantity: parse_decimal(t["quantity"].as_str().unwrap_or("0")).unwrap_or_default(),
                price: parse_decimal(t["price"].as_str().unwrap_or("0")).unwrap_or_default(),
                executed_at: t["executed_at"].as_str().unwrap_or("").into(),
            });
        }
    }
    Ok(rows)
}

/// Submits a new trade via IPC.
pub async fn add_transaction(
    symbol: &str,
    side: &str,
    quantity: &str,
    price: &str,
    fees: &str,
    executed_at: &str,
    note: Option<&str>,
) -> anyhow::Result<i64> {
    let data = send(
        Action::AddTransaction,
        serde_json::json!({
            "symbol": symbol,
            "side": side,
            "quantity": quantity,
            "price": price,
            "fees": fees,
            "executed_at": executed_at,
            "note": note,
        }),
    )
    .await?;
    Ok(data["id"].as_i64().unwrap_or(0))
}

/// Marks search hits that are already held in the portfolio.
pub fn mark_portfolio_hits(results: &mut [SearchResultRow], held: &[PositionRow]) {
    let held_syms: std::collections::HashSet<&str> =
        held.iter().map(|p| p.symbol.as_str()).collect();
    for r in results.iter_mut() {
        r.in_portfolio = held_syms.contains(r.symbol.as_str());
    }
}

/// Sort positions by score descending when requested.
pub fn sort_positions(rows: &mut [PositionRow], by_score: bool) {
    if by_score {
        rows.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.symbol.cmp(&b.symbol)));
    }
}
