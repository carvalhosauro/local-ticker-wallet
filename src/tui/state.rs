use super::models::{DetailData, LedgerRow, PositionRow, SearchResultRow};

/// Shared data fetched from the daemon and passed to screen renderers.
pub struct UiData {
    pub positions: Vec<PositionRow>,
    pub detail: Option<DetailData>,
    pub search_results: Vec<SearchResultRow>,
    pub ledger: Vec<LedgerRow>,
}

impl UiData {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            detail: None,
            search_results: Vec::new(),
            ledger: Vec::new(),
        }
    }
}

impl Default for UiData {
    fn default() -> Self {
        Self::new()
    }
}
