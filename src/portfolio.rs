//! CSV import/export of the portfolio. Stub — real implementation in Task 15.

/// Import transactions from a CSV file, recomputing snapshots as it goes.
pub fn import_csv(
    _db: &crate::storage::db::Db,
    _path: &str,
    _weights: &crate::config::ScoreWeights,
) -> anyhow::Result<usize> {
    Ok(0)
}

/// Export the current portfolio transactions to a CSV file.
pub fn export_csv(_db: &crate::storage::db::Db, _path: &str) -> anyhow::Result<usize> {
    Ok(0)
}
