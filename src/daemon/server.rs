use crate::config::Config;
use crate::core::types::{AssetId, Side, Trade};
use crate::daemon::recompute::recompute_asset;
use crate::ipc::{Action, ErrorCode, Request, Response};
use crate::providers::Chain;
use crate::storage::db::Db;

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

pub async fn handle(db: &Db, chain: &Chain, cfg: &Config, req: Request) -> Response {
    let id = req.id.clone();
    let result: anyhow::Result<serde_json::Value> = match req.action {
        Action::Ping => Ok(serde_json::json!({"pong": true})),

        Action::AddTransaction => (|| {
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
            let new_id = db.insert_transaction(&t)?;
            let _ = recompute_asset(db, &asset, &cfg.score_weights);
            Ok(serde_json::json!({"id": new_id}))
        })(),

        Action::ListTransactions => (|| {
            let asset = req.payload["symbol"].as_str().map(AssetId::b3);
            let txs = db.list_transactions(asset.as_ref())?;
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

        Action::DeleteTransaction => (|| {
            let id = req.payload["id"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("id required"))?;
            let removed = db.delete_transaction(id)?;
            Ok(serde_json::json!({"removed": removed}))
        })(),

        Action::GetPositions => (|| {
            let snaps = db.read_snapshots()?;
            let arr: Vec<_> = snaps.iter().map(snapshot_json).collect();
            Ok(serde_json::json!({"positions": arr}))
        })(),

        Action::GetPositionDetail => (|| {
            let asset = AssetId::b3(
                req.payload["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol required"))?,
            );
            let snap = recompute_asset(db, &asset, &cfg.score_weights)?;
            Ok(snapshot_json(&snap))
        })(),

        Action::RefreshNow => refresh(
            db,
            chain,
            cfg,
            req.payload["symbol"].as_str().map(AssetId::b3),
        )
        .await
        .map(|n| serde_json::json!({"refreshed": n})),

        Action::Search => {
            let q = req.payload["query"].as_str().unwrap_or("").to_string();
            match chain.search(&q).await {
                Ok(assets) => Ok(serde_json::json!({"results": assets})),
                Err(e) => Err(e),
            }
        }

        Action::Import => crate::portfolio::import_csv(
            db,
            req.payload["path"].as_str().unwrap_or(""),
            &cfg.score_weights,
        )
        .map(|n| serde_json::json!({"imported": n})),

        Action::Export => {
            crate::portfolio::export_csv(db, req.payload["path"].as_str().unwrap_or(""))
                .map(|n| serde_json::json!({"exported": n}))
        }
    };

    match result {
        Ok(data) => Response::ok(id, data),
        Err(e) => Response::err(id, ErrorCode::Internal, e.to_string()),
    }
}

async fn refresh(
    db: &Db,
    chain: &Chain,
    cfg: &Config,
    only: Option<AssetId>,
) -> anyhow::Result<usize> {
    let targets = match only {
        Some(a) => vec![a],
        None => db.distinct_held_assets()?,
    };
    let mut n = 0;
    for a in targets {
        if let Ok(q) = chain.quote(&a).await {
            db.upsert_quote(&q)?;
            n += 1;
        }
        if let Ok(c) = chain.history(&a).await {
            db.upsert_candles(&a, &c)?;
        }
        if let Ok(d) = chain.dividends(&a).await {
            db.upsert_dividends(&a, &d)?;
        }
        let _ = recompute_asset(db, &a, &cfg.score_weights);
    }
    Ok(n)
}
