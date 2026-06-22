use sha2::{Digest, Sha256};

pub fn checksum(sql: &str) -> String {
    let mut h = Sha256::new();
    h.update(sql.as_bytes());
    format!("{:x}", h.finalize())
}

pub const MIGRATIONS: &[(i32, &str)] = &[(
    1,
    r#"
    CREATE TABLE IF NOT EXISTS assets (
        symbol TEXT NOT NULL, exchange TEXT NOT NULL, name TEXT, kind TEXT, currency TEXT,
        last_seen TEXT,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE TABLE IF NOT EXISTS transactions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol TEXT NOT NULL, exchange TEXT NOT NULL, side TEXT NOT NULL,
        quantity TEXT NOT NULL, price TEXT NOT NULL, fees TEXT NOT NULL DEFAULT '0',
        executed_at TEXT NOT NULL, note TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE IF NOT EXISTS quotes (
        symbol TEXT NOT NULL, exchange TEXT NOT NULL,
        price TEXT, prev_close TEXT,
        day_high TEXT, day_low TEXT,
        currency TEXT, source TEXT, fetched_at TEXT,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE TABLE IF NOT EXISTS price_history (
        symbol TEXT NOT NULL, exchange TEXT NOT NULL, date TEXT NOT NULL,
        open TEXT, high TEXT, low TEXT, close TEXT,
        volume INTEGER,
        PRIMARY KEY (symbol, exchange, date)
    );
    CREATE TABLE IF NOT EXISTS dividends (
        symbol TEXT NOT NULL, exchange TEXT NOT NULL, ex_date TEXT NOT NULL, pay_date TEXT,
        amount_per_share TEXT, source TEXT,
        PRIMARY KEY (symbol, exchange, ex_date)
    );
    CREATE TABLE IF NOT EXISTS position_snapshots (
        symbol TEXT NOT NULL, exchange TEXT NOT NULL,
        quantity TEXT, avg_cost TEXT,
        invested TEXT, market_value TEXT,
        unrealized_pnl TEXT, unrealized_pnl_pct TEXT,
        realized_pnl TEXT, day_change_pct TEXT,
        score INTEGER, score_breakdown TEXT, computed_at TEXT,
        PRIMARY KEY (symbol, exchange)
    );
    CREATE TABLE IF NOT EXISTS search_cache (
        query TEXT PRIMARY KEY, results TEXT, fetched_at TEXT
    );
    "#,
)];
