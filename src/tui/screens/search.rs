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
                Span::raw(format_money_for_currency(p.price, &p.currency, fmt)),
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
    } else if app.search_preview_pending || results.get(selected).is_some() {
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

fn format_money_for_currency(
    price: rust_decimal::Decimal,
    currency: &str,
    fmt: format::FormatLocale,
) -> String {
    let formatted = format::format_price(price, fmt);
    if currency == "BRL" {
        format!("R$ {formatted}")
    } else {
        format!("{currency} {formatted}")
    }
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        name.to_string()
    } else {
        format!("{}…", name.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => {
            app.go_portfolio();
            app.search_query.clear();
            data.search_results.clear();
            data.search_preview = None;
            app.clear_search_preview_schedule();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            data.search_results.clear();
            data.search_preview = None;
            app.search_selected = 0;
            app.clear_search_preview_schedule();
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Char(c) if !c.is_control() => {
            app.search_query.push(c);
            data.search_results.clear();
            data.search_preview = None;
            app.search_selected = 0;
            app.clear_search_preview_schedule();
            app.schedule_search(SEARCH_DEBOUNCE);
        }
        KeyCode::Down => {
            if !data.search_results.is_empty() {
                app.search_selected = (app.search_selected + 1).min(data.search_results.len() - 1);
                schedule_preview_for_selection(app, data);
            }
        }
        KeyCode::Up => {
            if !data.search_results.is_empty() {
                app.search_selected = app.search_selected.saturating_sub(1);
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

/// Runs debounced provider search and quote preview fetches.
pub async fn tick(app: &mut App, data: &mut UiData) {
    tick_search(app, data).await;
    tick_preview(app, data).await;
}

async fn tick_search(app: &mut App, data: &mut UiData) {
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
        data.search_preview = None;
        return;
    }

    match client::search_assets(&query).await {
        Ok(mut results) => {
            client::mark_portfolio_hits(&mut results, &data.positions);
            client::sort_search_results(&mut results);
            data.search_results = results;
            if app.search_selected >= data.search_results.len() {
                app.search_selected = data.search_results.len().saturating_sub(1);
            }
            if !data.search_results.is_empty() {
                schedule_preview_for_selection(app, data);
            } else {
                data.search_preview = None;
            }
        }
        Err(e) => {
            data.search_results.clear();
            data.search_preview = None;
            app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_search)));
        }
    }
}

async fn tick_preview(app: &mut App, data: &mut UiData) {
    if !app.search_preview_pending {
        return;
    }
    let Some(deadline) = app.search_preview_deadline else {
        return;
    };
    if Instant::now() < deadline {
        return;
    }

    app.search_preview_pending = false;
    let Some(symbol) = app.search_preview_symbol.clone() else {
        return;
    };

    let Some(row) = data
        .search_results
        .iter()
        .find(|r| r.symbol == symbol)
        .cloned()
    else {
        return;
    };

    match client::fetch_quote_preview(&row).await {
        Ok(preview) => data.search_preview = Some(preview),
        Err(e) => {
            data.search_preview = None;
            app.show_toast(Toast::error(format!(
                "{}: {e}",
                app.bundle.search_preview_err
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locale;
    use crate::tui::app::App;
    use ratatui::{backend::TestBackend, Terminal};
    use rust_decimal_macros::dec;

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
