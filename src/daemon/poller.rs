use crate::config::Config;
use crate::providers::Chain;
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Background poller stub. Real implementation lands in Task 14.
pub async fn run_poller(_db: Arc<Mutex<Db>>, _chain: Arc<Chain>, _cfg: Arc<Config>) {
    // real impl in Task 14
}
