use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use std::time::{Duration, Instant};

use crate::core::format;
use crate::tui::app::{App, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::{SearchPreview, SearchResultRow};
use crate::tui::state::UiData;

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(400);
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(350);

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    results: &[SearchResultRow],
    preview: Option<&SearchPreview>,
) {
    let bundle = app.bundle;
    let query = &app.search_query;
    let selected = app.search_selected;

    let outer = ratatui::layout::Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
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
    frame.render_widget(query_block, outer[0]);

    let body = ratatui::layout::Layout::horizontal([
        Constraint::Percentage(58),
        Constraint::Percentage(42),
    ])
    .split(outer[1]);

    render_results_table(frame, body[0], app, results, selected);
    render_preview_panel(frame, body[1], app, results, selected, preview);
}

fn render_results_table(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    results: &[SearchResultRow],
    selected: usize,
) {
    let bundle = app.bundle;

    if results.is_empty() {
        let empty = Paragraph::new(bundle.search_no_results)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new([
        bundle.search_col_symbol,
        bundle.search_col_name,
        bundle.search_col_kind,
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
                Cell::from(format!("{} ({status})", truncate_name(&r.name, 28))),
                Cell::from(r.kind.clone()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(16),
        Constraint::Length(10),
    ];
    let table = Table::new(body, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(bundle.search_footer),
        );
    frame.render_widget(table, area);
}

fn render_preview_panel(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    results: &[SearchResultRow],
    selected: usize,
    preview: Option<&SearchPreview>,
) {
    let b = app.bundle;
    let fmt = app.fmt;
    let label = Style::default().add_modifier(Modifier::BOLD);

    let lines = if let Some(p) = preview {
        let status = if p.in_portfolio {
            b.search_in_portfolio
        } else {
            b.search_not_in_portfolio
        };
        vec![
            Line::from(vec![
                Span::styled(p.symbol.clone(), label),
                Span::raw(format!(" — {}", truncate_name(&p.name, 32))),
            ]),
            Line::from(Span::styled(status, Style::default().fg(Color::Cyan))),
            Line::from(""),
            Line::from(vec![
                Span::styled(b.search_preview_price, label),
                Span::raw(format::format_money_for_currency(p.price, &p.currency, fmt)),
            ]),
            Line::from(vec![
                Span::styled(b.search_preview_day, label),
                Span::raw(format::format_pct(p.day_change_pct, fmt)),
            ]),
            Line::from(vec![
                Span::styled(b.search_preview_kind, label),
                Span::raw(format!("{} · {}", p.kind, p.currency)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                b.search_preview_add_hint,
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else if preview_is_loading(app, results, selected) {
        vec![Line::from(b.search_preview_loading)]
    } else {
        vec![Line::from(b.search_preview_select)]
    };

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", b.search_preview_title)),
    );
    frame.render_widget(para, area);
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        name.to_string()
    } else {
        format!("{}…", name.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

fn preview_is_loading(app: &App, results: &[SearchResultRow], selected: usize) -> bool {
    if app.search_preview_pending {
        return true;
    }
    let Some(row) = results.get(selected) else {
        return false;
    };
    app.preview_loading_symbol.as_deref() == Some(row.symbol.as_str())
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => {
            app.go_portfolio();
            app.search_query.clear();
            data.search_results.clear();
            data.search_preview = None;
            app.invalidate_search_fetch();
            app.invalidate_preview_fetch();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            data.search_results.clear();
            data.search_preview = None;
            app.search_selected = 0;
            app.invalidate_search_fetch();
            app.invalidate_preview_fetch();
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Char(c) if !c.is_control() => {
            app.search_query.push(c);
            data.search_results.clear();
            data.search_preview = None;
            app.search_selected = 0;
            app.invalidate_search_fetch();
            app.invalidate_preview_fetch();
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Down => {
            if !data.search_results.is_empty() {
                app.search_selected = (app.search_selected + 1).min(data.search_results.len() - 1);
                data.search_preview = None;
                app.invalidate_preview_fetch();
                schedule_preview_for_selection(app, data);
            }
        }
        KeyCode::Up => {
            if !data.search_results.is_empty() {
                app.search_selected = app.search_selected.saturating_sub(1);
                data.search_preview = None;
                app.invalidate_preview_fetch();
                schedule_preview_for_selection(app, data);
            }
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            if let Some(row) = data.search_results.get(app.search_selected) {
                let price_hint = data
                    .search_preview
                    .as_ref()
                    .filter(|p| p.symbol == row.symbol)
                    .map(|p| p.price.to_string());
                app.open_add_transaction(Some(row.symbol.clone()), price_hint);
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

fn schedule_preview_for_selection(app: &mut App, data: &UiData) {
    if let Some(row) = data.search_results.get(app.search_selected) {
        app.schedule_search_preview(row.symbol.clone(), PREVIEW_DEBOUNCE);
    }
}

/// Runs debounced provider search and quote preview fetches without blocking the UI loop.
pub async fn tick(app: &mut App, data: &mut UiData) {
    poll_search_fetch(app, data).await;
    poll_preview_fetch(app, data).await;
    maybe_start_search_fetch(app);
    maybe_start_preview_fetch(app, data);
    // Let spawned IPC tasks make progress while the main loop is in crossterm poll/read.
    tokio::task::yield_now().await;
}

async fn poll_search_fetch(app: &mut App, data: &mut UiData) {
    let finished = app
        .search_inflight
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished);
    if !finished {
        return;
    }
    let Some(handle) = app.search_inflight.take() else {
        return;
    };
    let (gen, query, result) = match handle.await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("search fetch task panicked: {e}");
            return;
        }
    };

    if gen != app.search_fetch_gen {
        return;
    }

    match result {
        Ok(mut results) => {
            client::mark_portfolio_hits(&mut results, &data.positions);
            client::sort_search_results(&mut results);
            data.search_results = results;
            if app.search_selected >= data.search_results.len() {
                app.search_selected = data.search_results.len().saturating_sub(1);
            }
            if !data.search_results.is_empty() {
                data.search_preview = None;
                app.preview_inflight = None;
                schedule_preview_for_selection(app, data);
            } else {
                data.search_preview = None;
            }
        }
        Err(e) => {
            if query == app.search_query {
                data.search_results.clear();
                data.search_preview = None;
                app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_search)));
            }
        }
    }
}

async fn poll_preview_fetch(app: &mut App, data: &mut UiData) {
    let finished = app
        .preview_inflight
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished);
    if !finished {
        return;
    }
    let Some(handle) = app.preview_inflight.take() else {
        return;
    };
    let (gen, _symbol, result) = match handle.await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("preview fetch task panicked: {e}");
            app.preview_loading_symbol = None;
            return;
        }
    };

    app.preview_loading_symbol = None;

    if gen != app.preview_fetch_gen {
        return;
    }

    match result {
        Ok(preview) => {
            let still_selected = data
                .search_results
                .get(app.search_selected)
                .is_some_and(|r| r.symbol == preview.symbol);
            if still_selected {
                data.search_preview = Some(preview);
            }
        }
        Err(e) => {
            data.search_preview = None;
            app.show_toast(Toast::error(format!(
                "{}: {e}",
                app.bundle.search_preview_err
            )));
        }
    }
}

fn maybe_start_search_fetch(app: &mut App) {
    if app.search_inflight.is_some() {
        return;
    }
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
        return;
    }

    let gen = app.next_search_fetch_gen();
    app.search_inflight = Some(tokio::spawn(async move {
        let result = client::search_assets(&query).await;
        (gen, query, result)
    }));
}

fn maybe_start_preview_fetch(app: &mut App, data: &UiData) {
    if app.preview_inflight.is_some() {
        return;
    }
    if !app.search_preview_pending {
        return;
    }
    let Some(deadline) = app.search_preview_deadline else {
        return;
    };
    if Instant::now() < deadline {
        return;
    }

    let Some(row) = data.search_results.get(app.search_selected).cloned() else {
        return;
    };

    app.search_preview_pending = false;
    app.search_preview_symbol = Some(row.symbol.clone());
    app.preview_loading_symbol = Some(row.symbol.clone());

    let gen = app.next_preview_fetch_gen();
    let symbol = row.symbol.clone();
    app.preview_inflight = Some(tokio::spawn(async move {
        let result = client::fetch_quote_preview(&row).await;
        (gen, symbol, result)
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locale;
    use crate::tui::app::App;
    use ratatui::{backend::TestBackend, Terminal};
    use rust_decimal_macros::dec;

    #[test]
    fn preview_fetch_gen_single_increment_per_spawn() {
        let mut app = App::new(Locale::En);
        assert_eq!(app.preview_fetch_gen, 0);
        app.invalidate_preview_fetch();
        assert_eq!(app.preview_fetch_gen, 1);
        let g = app.next_preview_fetch_gen();
        assert_eq!(g, 2);
        assert_eq!(app.preview_fetch_gen, 2);
    }

    #[test]
    fn preview_loading_only_while_fetch_inflight() {
        let mut app = App::new(Locale::En);
        let results = vec![SearchResultRow {
            symbol: "BBAS3".into(),
            name: "Brasil".into(),
            kind: "EQUITY".into(),
            currency: "BRL".into(),
            in_portfolio: false,
        }];
        assert!(!preview_is_loading(&app, &results, 0));
        app.preview_loading_symbol = Some("BBAS3".into());
        assert!(preview_is_loading(&app, &results, 0));
        app.preview_loading_symbol = None;
        app.search_preview_pending = true;
        assert!(preview_is_loading(&app, &results, 0));
    }

    #[test]
    fn renders_search_with_preview_panel() {
        let app = App::new(Locale::PtBr);
        let results = vec![SearchResultRow {
            symbol: "PETR4".into(),
            name: "Petrobras".into(),
            kind: "EQUITY".into(),
            currency: "BRL".into(),
            in_portfolio: true,
        }];
        let preview = SearchPreview {
            symbol: "PETR4".into(),
            name: "Petrobras".into(),
            kind: "EQUITY".into(),
            currency: "BRL".into(),
            in_portfolio: true,
            price: dec!(38.42),
            day_change_pct: dec!(1.25),
        };
        let backend = TestBackend::new(120, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, f.area(), &app, &results, Some(&preview)))
            .unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("PETR4"));
        assert!(text.contains("Prévia"));
        assert!(text.contains("na carteira"));
    }
}
