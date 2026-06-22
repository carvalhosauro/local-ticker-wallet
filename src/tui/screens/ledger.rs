use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::core::format;
use crate::core::types::Side;
use crate::tui::app::{App, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::LedgerRow;
use crate::tui::state::UiData;

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, rows: &[LedgerRow]) {
    let bundle = app.bundle;
    let fmt = app.fmt;
    let selected = app.ledger_selected;

    if rows.is_empty() {
        let empty = Paragraph::new(bundle.ledger_empty).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} — {}", bundle.ledger_title, bundle.ledger_footer)),
        );
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new([
        bundle.ledger_col_id,
        bundle.ledger_col_symbol,
        bundle.ledger_col_side,
        bundle.ledger_col_qty,
        bundle.ledger_col_price,
        bundle.ledger_col_date,
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let body: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let side = match r.side {
                Side::Buy => bundle.side_buy,
                Side::Sell => bundle.side_sell,
            };
            Row::new(vec![
                Cell::from(r.id.to_string()),
                Cell::from(r.symbol.clone()),
                Cell::from(side),
                Cell::from(format::format_quantity(r.quantity, fmt)),
                Cell::from(format::format_price(r.price, fmt)),
                Cell::from(r.executed_at.clone()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(12),
    ];
    let table = Table::new(body, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} — {}", bundle.ledger_title, bundle.ledger_footer)),
        );
    frame.render_widget(table, area);
}

pub fn handle_key(app: &mut App, data: &UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => app.go_portfolio(),
        KeyCode::Down => {
            if !data.ledger.is_empty() {
                app.ledger_selected = (app.ledger_selected + 1).min(data.ledger.len() - 1);
            }
        }
        KeyCode::Up => {
            app.ledger_selected = app.ledger_selected.saturating_sub(1);
        }
        _ => {}
    }
    KeyOutcome::Continue
}

/// Loads ledger rows from the daemon if not yet cached.
pub async fn ensure_loaded(app: &mut App, data: &mut UiData) {
    if !data.ledger.is_empty() {
        return;
    }
    match client::fetch_ledger().await {
        Ok(rows) => data.ledger = rows,
        Err(e) => app.show_toast(Toast::error(format!(
            "{}: {e}",
            app.bundle.err_fetch_ledger
        ))),
    }
}
