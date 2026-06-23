use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::core::format;
use crate::core::types::AssetId;
use crate::tui::app::{App, Screen, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::{score_color, PositionRow};
use crate::tui::state::UiData;

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, rows: &[PositionRow]) {
    let bundle = app.bundle;
    let fmt = app.fmt;
    let selected = app.portfolio_selected;

    let header = Row::new([
        bundle.col_symbol,
        bundle.col_qty,
        bundle.col_avg,
        bundle.col_mkt_value,
        bundle.col_day_pct,
        bundle.col_pnl_pct,
        bundle.col_score,
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let body: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let currency = AssetId::b3(&r.symbol).currency();
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(r.symbol.clone()),
                Cell::from(format::format_quantity(r.quantity, fmt)),
                Cell::from(format::format_price(r.avg_cost, fmt)),
                Cell::from(format::format_money_for_currency(r.market_value, currency, fmt)),
                Cell::from(format::format_pct(r.day_change_pct, fmt)),
                Cell::from(format::format_pct(r.unrealized_pnl_pct, fmt)),
                Cell::from(format::format_score(r.score))
                    .style(Style::default().fg(score_color(r.score))),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(6),
    ];
    let title = if app.sort_by_score {
        format!(
            "{} — {} · {}",
            bundle.app_title, bundle.portfolio_sort_active, bundle.portfolio_footer
        )
    } else {
        format!("{} — {}", bundle.app_title, bundle.portfolio_footer)
    };
    let table = Table::new(body, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(table, area);
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Char('r') => {
            if let Err(e) = client::refresh_all().await {
                app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_refresh)));
            } else {
                match client::fetch_positions().await {
                    Ok(rows) => data.positions = rows,
                    Err(e) => app.show_toast(Toast::error(format!(
                        "{}: {e}",
                        app.bundle.err_fetch_positions
                    ))),
                }
            }
        }
        KeyCode::Char('o') => {
            app.sort_by_score = !app.sort_by_score;
            if !app.sort_by_score {
                if let Ok(rows) = client::fetch_positions().await {
                    data.positions = rows;
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(row) = data.positions.get(app.portfolio_selected) {
                app.open_add_transaction(
                    Some(row.symbol.clone()),
                    Some(row.avg_cost.to_string()),
                );
            } else {
                app.open_add_transaction(None, None);
            }
        }
        KeyCode::Down => {
            if !data.positions.is_empty() {
                app.portfolio_selected = (app.portfolio_selected + 1).min(data.positions.len() - 1);
            }
        }
        KeyCode::Up => {
            app.portfolio_selected = app.portfolio_selected.saturating_sub(1);
        }
        KeyCode::Enter => {
            if let Some(row) = data.positions.get(app.portfolio_selected) {
                match client::fetch_detail(&row.symbol).await {
                    Ok(detail) => {
                        data.detail = Some(detail);
                        app.screen = Screen::Detail;
                    }
                    Err(e) => app.show_toast(Toast::error(format!(
                        "{}: {e}",
                        app.bundle.err_fetch_detail
                    ))),
                }
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locale;
    use crate::tui::app::App;
    use ratatui::{backend::TestBackend, Terminal};
    use rust_decimal_macros::dec;

    fn buffer_text<F>(width: u16, height: u16, draw: F) -> String
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(draw).unwrap();
        let buf = term.backend().buffer().clone();
        buf.content().iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn renders_symbol_and_score() {
        let app = App::new(Locale::En);
        let rows = vec![PositionRow {
            symbol: "PETR4".into(),
            quantity: dec!(100),
            avg_cost: dec!(10),
            market_value: dec!(1200),
            day_change_pct: dec!(1.5),
            unrealized_pnl_pct: dec!(20),
            score: 73,
        }];
        let text = buffer_text(100, 12, |f| render(f, f.area(), &app, &rows));
        assert!(text.contains("PETR4"));
        assert!(text.contains("73"));
        assert!(text.contains("R$ 1,200.00"));
    }

    #[test]
    fn shows_sort_indicator_when_active() {
        let mut app = App::new(Locale::En);
        app.sort_by_score = true;
        let rows = vec![PositionRow {
            symbol: "PETR4".into(),
            quantity: dec!(100),
            avg_cost: dec!(10),
            market_value: dec!(1200),
            day_change_pct: dec!(1.5),
            unrealized_pnl_pct: dec!(20),
            score: 73,
        }];
        let text = buffer_text(100, 12, |f| render(f, f.area(), &app, &rows));
        assert!(text.contains("sorted by score"));
    }
}
