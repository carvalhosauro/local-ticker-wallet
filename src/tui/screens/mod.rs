pub mod detail;
pub mod ledger;
pub mod portfolio;
pub mod search;

use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};
use crate::tui::input::KeyOutcome;
use crate::tui::state::UiData;

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &App, data: &UiData) {
    match app.screen {
        Screen::Portfolio => portfolio::render(frame, area, app, &data.positions),
        Screen::Detail => {
            if let Some(detail) = &data.detail {
                detail::render(frame, area, app, detail);
            }
        }
        Screen::Search => search::render(
            frame,
            area,
            app,
            &data.search_results,
            data.search_preview.as_ref(),
        ),
        Screen::Ledger => ledger::render(frame, area, app, &data.ledger),
    }
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    if app.has_overlay() {
        if matches!(code, KeyCode::Char('q')) {
            return KeyOutcome::Quit;
        }
        return crate::tui::overlays::handle_key(app, data, code).await;
    }

    if let Some(outcome) = handle_global_key(app, data, code).await {
        return outcome;
    }

    match app.screen {
        Screen::Portfolio => portfolio::handle_key(app, data, code).await,
        Screen::Detail => detail::handle_key(app, data, code).await,
        Screen::Search => search::handle_key(app, data, code).await,
        Screen::Ledger => ledger::handle_key(app, data, code),
    }
}

pub async fn tick(app: &mut App, data: &mut UiData) {
    if app.screen == Screen::Search {
        search::tick(app, data).await;
    }
}

async fn handle_global_key(
    app: &mut App,
    data: &mut UiData,
    code: KeyCode,
) -> Option<KeyOutcome> {
    match code {
        KeyCode::Char('q') => Some(KeyOutcome::Quit),
        KeyCode::Char('1') => {
            app.go_portfolio();
            Some(KeyOutcome::Continue)
        }
        KeyCode::Char('2') | KeyCode::Char('/') => {
            app.go_search();
            Some(KeyOutcome::Continue)
        }
        KeyCode::Char('3') => {
            app.go_ledger();
            ledger::ensure_loaded(app, data).await;
            Some(KeyOutcome::Continue)
        }
        _ => None,
    }
}
