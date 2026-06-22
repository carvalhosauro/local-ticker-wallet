use crate::tui::models::LedgerRow;

/// Confirmation dialog before deleting a ledger transaction.
#[derive(Debug, Clone)]
pub struct ConfirmDelete {
    pub row: LedgerRow,
    pub error: Option<String>,
}

impl ConfirmDelete {
    pub fn new(row: LedgerRow) -> Self {
        Self { row, error: None }
    }
}
