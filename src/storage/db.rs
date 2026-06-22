use crate::storage::schema::{checksum, MIGRATIONS};
use anyhow::Context;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        let conn = Connection::open(path).context("open sqlite")?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> anyhow::Result<Db> {
        let conn = Connection::open_in_memory().context("open in-memory sqlite")?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT, checksum TEXT);",
        )?;
        for (version, sql) in MIGRATIONS {
            let sum = checksum(sql);
            let recorded: Option<String> = self
                .conn
                .query_row(
                    "SELECT checksum FROM schema_migrations WHERE version = ?1",
                    rusqlite::params![version],
                    |r| r.get(0),
                )
                .optional()?;
            match recorded {
                Some(existing) if existing != sum => {
                    anyhow::bail!(
                        "migration {} checksum drift: recorded {} but binary expects {}",
                        version, existing, sum
                    );
                }
                Some(_) => continue, // already applied, matches
                None => {
                    self.conn.execute_batch(sql)?;
                    self.conn.execute(
                        "INSERT INTO schema_migrations (version, applied_at, checksum) VALUES (?1, datetime('now'), ?2)",
                        rusqlite::params![version, sum],
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn schema_version(&self) -> anyhow::Result<i32> {
        let v: i32 = self
            .conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| r.get(0))?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_migrates_to_latest_version() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
    }

    #[test]
    fn migrations_are_idempotent_on_reopen() {
        // In-memory cannot reopen; use a temp file path.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("w.db");
        { let _ = Db::open(&path).unwrap(); }
        let db2 = Db::open(&path).unwrap();
        assert_eq!(db2.schema_version().unwrap(), 1);
    }
}
