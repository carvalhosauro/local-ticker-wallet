use local_ticker_wallet::config::Config;
use local_ticker_wallet::daemon::server::handle;
use local_ticker_wallet::ipc::{Action, Request};
use local_ticker_wallet::providers::Chain;
use local_ticker_wallet::storage::db::Db;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn add_then_get_positions_via_handler() {
    let db = Arc::new(Mutex::new(Db::open_in_memory().unwrap()));
    let chain = Chain::new(vec![]); // no network needed for this path
    let cfg = Config::default();

    let add = Request::new(
        Action::AddTransaction,
        serde_json::json!({
            "symbol": "PETR4", "side": "BUY", "quantity": "100", "price": "10.00", "fees": "0", "executed_at": "2026-01-01"
        }),
    );
    let r = handle(&db, &chain, &cfg, add).await;
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["status"], "ok");

    let get = Request::new(Action::GetPositions, serde_json::json!({}));
    let r2 = handle(&db, &chain, &cfg, get).await;
    let v2 = serde_json::to_value(&r2).unwrap();
    assert_eq!(v2["status"], "ok");
    let positions = v2["data"]["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["symbol"], "PETR4");
    // Compare numerically: storage preserves the stored string ("100"), so a
    // literal "100.00000000" would be a false mismatch. Parse and compare values.
    assert_eq!(
        positions[0]["quantity"]
            .as_str()
            .unwrap()
            .parse::<rust_decimal::Decimal>()
            .unwrap(),
        rust_decimal_macros::dec!(100)
    );
}

#[tokio::test]
async fn add_transaction_with_bad_date_yields_bad_request() {
    let db = Arc::new(Mutex::new(Db::open_in_memory().unwrap()));
    let chain = Chain::new(vec![]); // no network needed for this path
    let cfg = Config::default();

    // Invalid `executed_at` -> chrono parse failure inside the AddTransaction
    // arm, which the taxonomy must classify as BAD_REQUEST.
    let add = Request::new(
        Action::AddTransaction,
        serde_json::json!({
            "symbol": "PETR4", "side": "BUY", "quantity": "100", "price": "10.00", "fees": "0", "executed_at": "not-a-date"
        }),
    );
    let r = handle(&db, &chain, &cfg, add).await;
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "BAD_REQUEST");
}
