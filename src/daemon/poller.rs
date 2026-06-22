use crate::config::Config;
use crate::providers::Chain;
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub async fn run_poller(db: Arc<Mutex<Db>>, chain: Arc<Chain>, cfg: Arc<Config>) {
    let mut last_daily: Option<chrono::NaiveDate> = None;
    loop {
        let now = super::market_hours::now_sp();
        if super::market_hours::is_open(now) {
            refresh_quotes(&db, &chain, &cfg).await;
            sleep(Duration::from_secs(cfg.poll_interval_secs.max(5))).await;
        } else {
            let today = now.date_naive();
            if last_daily != Some(today) {
                refresh_history(&db, &chain, &cfg).await;
                last_daily = Some(today);
            }
            sleep(Duration::from_secs(300)).await;
        }
    }
}

async fn refresh_quotes(db: &Arc<Mutex<Db>>, chain: &Arc<Chain>, cfg: &Arc<Config>) {
    let held = {
        let d = db.lock().await;
        d.distinct_held_assets().unwrap_or_default()
    };
    for a in held {
        if let Ok(q) = chain.quote(&a).await {
            let d = db.lock().await;
            let _ = d.upsert_quote(&q);
            let _ = crate::daemon::recompute::recompute_asset(&d, &a, &cfg.score_weights);
        }
    }
}

async fn refresh_history(db: &Arc<Mutex<Db>>, chain: &Arc<Chain>, cfg: &Arc<Config>) {
    let held = {
        let d = db.lock().await;
        d.distinct_held_assets().unwrap_or_default()
    };
    for a in held {
        if let Ok(c) = chain.history(&a).await {
            let d = db.lock().await;
            let _ = d.upsert_candles(&a, &c);
        }
        if let Ok(divs) = chain.dividends(&a).await {
            let d = db.lock().await;
            let _ = d.upsert_dividends(&a, &divs);
        }
        let d = db.lock().await;
        let _ = crate::daemon::recompute::recompute_asset(&d, &a, &cfg.score_weights);
    }
}
