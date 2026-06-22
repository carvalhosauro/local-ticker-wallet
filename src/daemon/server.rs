use crate::config::Config;
use crate::core::types::{AssetId, Side, Trade};
use crate::daemon::recompute::recompute_asset;
use crate::ipc::{Action, ErrorCode, Request, Response};
use crate::providers::Chain;
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::sync::Mutex;

fn snapshot_json(s: &crate::storage::queries::PositionSnapshot) -> serde_json::Value {
    serde_json::json!({
        "symbol": s.asset.symbol, "exchange": s.asset.exchange,
        "quantity": s.quantity.to_string(), "avg_cost": s.avg_cost.to_string(),
        "invested": s.invested.to_string(), "market_value": s.market_value.to_string(),
        "unrealized_pnl": s.unrealized_pnl.to_string(), "unrealized_pnl_pct": s.unrealized_pnl_pct.to_string(),
        "realized_pnl": s.realized_pnl.to_string(), "day_change_pct": s.day_change_pct.to_string(),
        "score": s.score, "score_breakdown": s.score_breakdown, "computed_at": s.computed_at.to_string(),
    })
}

/// Per-action result carrying the `ErrorCode` the failure should map to.
/// The terminal match in `handle` turns this into a `Response`, so each arm
/// classifies its own error rather than collapsing everything to `Internal`.
type ActionResult = Result<serde_json::Value, (ErrorCode, String)>;

/// Tag an `anyhow::Result` with the `ErrorCode` to report if it failed.
fn classify<T>(r: anyhow::Result<T>, code: ErrorCode) -> Result<T, (ErrorCode, String)> {
    r.map_err(|e| (code, e.to_string()))
}

pub async fn handle(db: &Arc<Mutex<Db>>, chain: &Chain, cfg: &Config, req: Request) -> Response {
    let id = req.id.clone();
    let result: ActionResult = match req.action {
        Action::Ping => Ok(serde_json::json!({"pong": true})),

        Action::AddTransaction => {
            // Pure DB work: hold the lock only for the duration of this arm; no
            // `.await` happens while the guard is alive.
            let d = db.lock().await;
            // Bad input/payload/parse failures classify as BadRequest.
            classify(
                (|| {
                    let p = &req.payload;
                    let asset = AssetId::b3(
                        p["symbol"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("symbol required"))?,
                    );
                    let side = match p["side"].as_str() {
                        Some("SELL") => Side::Sell,
                        _ => Side::Buy,
                    };
                    let t = Trade {
                        id: 0,
                        asset: asset.clone(),
                        side,
                        quantity: p["quantity"].as_str().unwrap_or("0").parse()?,
                        price: p["price"].as_str().unwrap_or("0").parse()?,
                        fees: p["fees"].as_str().unwrap_or("0").parse()?,
                        executed_at: chrono::NaiveDate::parse_from_str(
                            p["executed_at"].as_str().unwrap_or(""),
                            "%Y-%m-%d",
                        )?,
                        note: p["note"].as_str().map(|s| s.to_string()),
                    };
                    let new_id = d.insert_transaction(&t)?;
                    if let Err(e) = recompute_asset(&d, &asset, &cfg.score_weights) {
                        eprintln!("warn: recompute {} failed: {e}", asset.symbol);
                    }
                    Ok(serde_json::json!({"id": new_id}))
                })(),
                ErrorCode::BadRequest,
            )
        }

        Action::ListTransactions => {
            let d = db.lock().await;
            classify(
                (|| {
                    let asset = req.payload["symbol"].as_str().map(AssetId::b3);
                    let txs = d.list_transactions(asset.as_ref())?;
                    let arr: Vec<_> = txs
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "id": t.id, "symbol": t.asset.symbol, "side": match t.side { Side::Buy=>"BUY", Side::Sell=>"SELL" },
                                "quantity": t.quantity.to_string(), "price": t.price.to_string(), "fees": t.fees.to_string(),
                                "executed_at": t.executed_at.to_string(), "note": t.note,
                            })
                        })
                        .collect();
                    Ok(serde_json::json!({"transactions": arr}))
                })(),
                ErrorCode::BadRequest,
            )
        }

        Action::DeleteTransaction => {
            let d = db.lock().await;
            classify(
                (|| {
                    let tx_id = req.payload["id"]
                        .as_i64()
                        .ok_or_else(|| anyhow::anyhow!("id required"))?;
                    let removed = d.delete_transaction(tx_id)?;
                    Ok(serde_json::json!({"removed": removed}))
                })(),
                ErrorCode::BadRequest,
            )
        }

        Action::GetPositions => {
            let d = db.lock().await;
            // Genuine unexpected failures fall back to Internal.
            classify(
                (|| {
                    let snaps = d.read_snapshots()?;
                    let arr: Vec<_> = snaps.iter().map(snapshot_json).collect();
                    Ok(serde_json::json!({"positions": arr}))
                })(),
                ErrorCode::Internal,
            )
        }

        Action::GetPositionDetail => {
            let d = db.lock().await;
            classify(
                (|| {
                    let asset = AssetId::b3(
                        req.payload["symbol"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("symbol required"))?,
                    );
                    let snap = recompute_asset(&d, &asset, &cfg.score_weights)?;
                    Ok(snapshot_json(&snap))
                })(),
                ErrorCode::Internal,
            )
        }

        Action::RefreshNow => classify(
            refresh(
                db,
                chain,
                cfg,
                req.payload["symbol"].as_str().map(AssetId::b3),
            )
            .await
            .map(|n| serde_json::json!({"refreshed": n})),
            ErrorCode::ProviderDown,
        ),

        Action::Search => {
            // No Db lock: this only does provider network I/O.
            let q = req.payload["query"].as_str().unwrap_or("").to_string();
            classify(
                chain
                    .search(&q)
                    .await
                    .map(|assets| serde_json::json!({"results": assets})),
                ErrorCode::ProviderDown,
            )
        }

        Action::Import => {
            let d = db.lock().await;
            classify(
                crate::portfolio::import_csv(
                    &d,
                    req.payload["path"].as_str().unwrap_or(""),
                    &cfg.score_weights,
                )
                .map(|n| serde_json::json!({"imported": n})),
                ErrorCode::BadRequest,
            )
        }

        Action::Export => {
            let d = db.lock().await;
            classify(
                crate::portfolio::export_csv(&d, req.payload["path"].as_str().unwrap_or(""))
                    .map(|n| serde_json::json!({"exported": n})),
                ErrorCode::BadRequest,
            )
        }
    };

    match result {
        Ok(data) => Response::ok(id, data),
        Err((code, message)) => Response::err(id, code, message),
    }
}

async fn refresh(
    db: &Arc<Mutex<Db>>,
    chain: &Chain,
    cfg: &Config,
    only: Option<AssetId>,
) -> anyhow::Result<usize> {
    // Read the target list under a short-lived lock, then drop the guard so all
    // network I/O below runs without holding the Db mutex.
    let targets = match only {
        Some(a) => vec![a],
        None => {
            let d = db.lock().await;
            d.distinct_held_assets()?
        }
    };
    let mut n = 0;
    for a in targets {
        // Network calls happen with NO Db guard held.
        let quote = chain.quote(&a).await.ok();
        let candles = chain.history(&a).await.ok();
        let dividends = chain.dividends(&a).await.ok();

        // Re-acquire the lock only to persist + recompute.
        let d = db.lock().await;
        if let Some(q) = quote {
            d.upsert_quote(&q)?;
            n += 1;
        }
        if let Some(c) = candles {
            d.upsert_candles(&a, &c)?;
        }
        if let Some(divs) = dividends {
            d.upsert_dividends(&a, &divs)?;
        }
        let _ = recompute_asset(&d, &a, &cfg.score_weights);
    }
    Ok(n)
}
