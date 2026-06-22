pub mod market_hours;
pub mod poller;
pub mod recompute;
pub mod server;

use crate::config::Config;
use crate::ipc::{self, Request};
use crate::providers::{AnyProvider, Chain};
use crate::storage::db::Db;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::UnixListener;
use tokio::sync::Mutex;

pub fn build_chain(cfg: &Config) -> Chain {
    Chain::new(vec![
        AnyProvider::Yahoo(crate::providers::yahoo::YahooProvider::default_base()),
        AnyProvider::Brapi(crate::providers::brapi::BrapiProvider::default_base(
            cfg.brapi_token.clone(),
        )),
    ])
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let sock = crate::paths::socket_path();
    let _ = std::fs::remove_file(&sock); // clear stale socket
    let db = Arc::new(Mutex::new(Db::open(&crate::paths::data_db())?));
    let chain = Arc::new(build_chain(&cfg));
    let cfg = Arc::new(cfg);

    // Poller task
    {
        let (db, chain, cfg) = (db.clone(), chain.clone(), cfg.clone());
        tokio::spawn(async move {
            poller::run_poller(db, chain, cfg).await;
        });
    }

    let listener = UnixListener::bind(&sock)?;
    loop {
        let (stream, _) = listener.accept().await?;
        let (db, chain, cfg) = (db.clone(), chain.clone(), cfg.clone());
        // The Db guard is never held across an `.await` (network I/O happens
        // outside the lock), so the per-connection future is `Send` and can be
        // spawned; DB access is still serialized by the single `Mutex`.
        tokio::spawn(async move {
            handle_conn(stream, db, chain, cfg).await;
        });
    }
}

async fn handle_conn(
    stream: tokio::net::UnixStream,
    db: Arc<Mutex<Db>>,
    chain: Arc<Chain>,
    cfg: Arc<Config>,
) {
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    while let Ok(Some(line)) = ipc::read_line(&mut reader).await {
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let resp = server::handle(&db, &chain, &cfg, req).await;
        if ipc::write_msg(&mut w, &resp).await.is_err() {
            break;
        }
    }
}
