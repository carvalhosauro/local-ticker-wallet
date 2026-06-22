use crate::config::Locale;
use crate::core::format::FormatLocale;
use crate::i18n::Bundle;
use crate::tui::overlays::add_transaction::AddTransactionForm;

/// Active modal overlay (at most one).
#[derive(Debug, Clone)]
pub enum Overlay {
    AddTransaction(AddTransactionForm),
}

/// Which primary screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Portfolio,
    Detail,
    Search,
    Ledger,
}

/// Transient UI feedback.
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub ticks_left: u8,
}

impl Toast {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            ticks_left: 25,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
            ticks_left: 20,
        }
    }

    pub fn tick(&mut self) {
        self.ticks_left = self.ticks_left.saturating_sub(1);
    }

    pub fn alive(&self) -> bool {
        self.ticks_left > 0
    }
}

use std::time::{Duration, Instant};

/// Central TUI state.
pub struct App {
    pub locale: Locale,
    pub fmt: FormatLocale,
    pub bundle: &'static Bundle,
    pub screen: Screen,
    pub toast: Option<Toast>,
    pub portfolio_selected: usize,
    pub search_query: String,
    pub search_selected: usize,
    pub ledger_selected: usize,
    pub sort_by_score: bool,
    pub search_pending: bool,
    pub search_deadline: Option<Instant>,
    pub search_preview_pending: bool,
    pub search_preview_deadline: Option<Instant>,
    pub search_preview_symbol: Option<String>,
    pub overlay: Option<Overlay>,
}

impl App {
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            fmt: FormatLocale::from(locale),
            bundle: crate::i18n::bundle(locale),
            screen: Screen::Portfolio,
            toast: None,
            portfolio_selected: 0,
            search_query: String::new(),
            search_selected: 0,
            ledger_selected: 0,
            sort_by_score: false,
            search_pending: false,
            search_deadline: None,
            search_preview_pending: false,
            search_preview_deadline: None,
            search_preview_symbol: None,
            overlay: None,
        }
    }

    pub fn schedule_search(&mut self, debounce: Duration) {
        self.search_pending = true;
        self.search_deadline = Some(Instant::now() + debounce);
    }

    pub fn schedule_search_preview(&mut self, symbol: impl Into<String>, debounce: Duration) {
        self.search_preview_pending = true;
        self.search_preview_symbol = Some(symbol.into());
        self.search_preview_deadline = Some(Instant::now() + debounce);
    }

    pub fn clear_search_preview_schedule(&mut self) {
        self.search_preview_pending = false;
        self.search_preview_deadline = None;
        self.search_preview_symbol = None;
    }

    pub fn show_toast(&mut self, toast: Toast) {
        self.toast = Some(toast);
    }

    pub fn tick_toast(&mut self) {
        if let Some(t) = &mut self.toast {
            t.tick();
            if !t.alive() {
                self.toast = None;
            }
        }
    }

    pub fn go_portfolio(&mut self) {
        self.screen = Screen::Portfolio;
    }

    pub fn go_search(&mut self) {
        self.screen = Screen::Search;
    }

    pub fn go_ledger(&mut self) {
        self.screen = Screen::Ledger;
    }

    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    pub fn open_add_transaction(
        &mut self,
        symbol: Option<String>,
        price_hint: Option<String>,
    ) {
        self.overlay = Some(Overlay::AddTransaction(AddTransactionForm::new(
            symbol, price_hint,
        )));
    }

    pub fn close_overlay(&mut self) {
        self.overlay = None;
    }
}
