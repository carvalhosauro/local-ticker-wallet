use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use std::time::{Duration, Instant};

use crate::tui::app::{App, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::SearchResultRow;
use crate::tui::state::UiData;

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(400);

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, results: &[SearchResultRow]) {
    let bundle = app.bundle;
    let query = &app.search_query;
    let selected = app.search_selected;

    let chunks = ratatui::layout::Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
    ])
    .split(area);

    let query_line = if query.is_empty() {
        bundle.search_placeholder.to_string()
    } else {
        format!("{query}_")
    };
    let query_block = Paragraph::new(query_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(bundle.search_title),
    );
    frame.render_widget(query_block, chunks[0]);

    if results.is_empty() {
        let empty = Paragraph::new(bundle.search_no_results)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let header = Row::new([
        bundle.search_col_symbol,
        bundle.search_col_name,
        bundle.search_col_kind,
        bundle.search_col_currency,
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let body: Vec<Row> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let status = if r.in_portfolio {
                bundle.search_in_portfolio
            } else {
                bundle.search_not_in_portfolio
            };
            Row::new(vec![
                Cell::from(r.symbol.clone()),
                Cell::from(format!("{} ({status})", r.name)),
                Cell::from(r.kind.clone()),
                Cell::from(r.currency.clone()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(8),
    ];
    let table = Table::new(body, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(bundle.search_footer),
        );
    frame.render_widget(table, chunks[1]);
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => {
            app.go_portfolio();
            app.search_query.clear();
            data.search_results.clear();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            data.search_results.clear();
            app.search_selected = 0;
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Char(c) if !c.is_control() => {
            app.search_query.push(c);
            data.search_results.clear();
            app.search_selected = 0;
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Down => {
            if !data.search_results.is_empty() {
                app.search_selected = (app.search_selected + 1).min(data.search_results.len() - 1);
            }
        }
        KeyCode::Up => {
            app.search_selected = app.search_selected.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            if let Some(row) = data.search_results.get(app.search_selected) {
                app.open_add_transaction(Some(row.symbol.clone()), None);
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

/// Runs debounced provider search when the query deadline has passed.
pub async fn tick(app: &mut App, data: &mut UiData) {
    if !app.search_pending {
        return;
    }
    let Some(deadline) = app.search_deadline else {
        return;
    };
    if Instant::now() < deadline {
        return;
    }

    app.search_pending = false;
    let query = app.search_query.clone();
    if query.is_empty() {
        data.search_results.clear();
        return;
    }

    match client::search_assets(&query).await {
        Ok(mut results) => {
            client::mark_portfolio_hits(&mut results, &data.positions);
            data.search_results = results;
            if app.search_selected >= data.search_results.len() {
                app.search_selected = data.search_results.len().saturating_sub(1);
            }
        }
        Err(e) => {
            data.search_results.clear();
            app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_search)));
        }
    }
}
