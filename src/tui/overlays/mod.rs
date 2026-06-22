pub mod add_transaction;
pub mod confirm_delete;

use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::core::format;
use crate::core::types::Side;
use crate::tui::app::{App, Overlay, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::overlays::add_transaction::{validate, AddField};
use crate::tui::state::UiData;

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    match app.overlay.as_ref() {
        Some(Overlay::AddTransaction(form)) => render_add_transaction(frame, area, app, form),
        Some(Overlay::ConfirmDelete(dialog)) => render_confirm_delete(frame, area, app, dialog),
        None => {}
    }
}

fn render_add_transaction(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    form: &add_transaction::AddTransactionForm,
) {
    let b = app.bundle;
    frame.render_widget(Clear, area);
    let popup = centered_rect(62, 70, area);

    let mut lines = vec![
        field_line(b.add_tx_label_symbol, &form.symbol, form.focused == AddField::Symbol),
        side_line(
            b.add_tx_label_side,
            form.side,
            b.side_buy,
            b.side_sell,
            form.focused == AddField::Side,
        ),
        field_line(
            b.add_tx_label_quantity,
            &form.quantity,
            form.focused == AddField::Quantity,
        ),
        field_line(b.add_tx_label_price, &form.price, form.focused == AddField::Price),
        field_line(b.add_tx_label_date, &form.date, form.focused == AddField::Date),
        field_line(b.add_tx_label_fees, &form.fees, form.focused == AddField::Fees),
        field_line(b.add_tx_label_note, &form.note, form.focused == AddField::Note),
    ];

    if let Some(err) = &form.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        b.add_tx_footer,
        Style::default().fg(Color::DarkGray),
    )));

    let title = format!(" {} ", b.add_tx_title);
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title)
            .title_alignment(Alignment::Center),
    );
    frame.render_widget(para, popup);
}

fn render_confirm_delete(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    dialog: &confirm_delete::ConfirmDelete,
) {
    let b = app.bundle;
    let fmt = app.fmt;
    let row = &dialog.row;
    let side = match row.side {
        Side::Buy => b.side_buy,
        Side::Sell => b.side_sell,
    };

    frame.render_widget(Clear, area);
    let popup = centered_rect(58, 40, area);

    let mut lines = vec![
        Line::from(Span::styled(b.delete_confirm_prompt, Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!(
            "#{} {} {}",
            row.id, row.symbol, side
        )),
        Line::from(format!(
            "{} @ {} · {}",
            format::format_quantity(row.quantity, fmt),
            format::format_price(row.price, fmt),
            row.executed_at
        )),
    ];
    if let Some(err) = &dialog.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        b.delete_confirm_footer,
        Style::default().fg(Color::DarkGray),
    )));

    let title = format!(" {} ", b.delete_confirm_title);
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(title)
            .title_alignment(Alignment::Center),
    );
    frame.render_widget(para, popup);
}

fn field_line(label: &str, value: &str, focused: bool) -> Line<'static> {
    let label_style = Style::default().add_modifier(Modifier::BOLD);
    let value_style = if focused {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::Yellow)
    } else {
        Style::default()
    };
    let display = if focused && value.is_empty() {
        "_".to_string()
    } else if focused {
        format!("{value}_")
    } else {
        value.to_string()
    };
    Line::from(vec![
        Span::styled(label.to_string(), label_style),
        Span::styled(display, value_style),
    ])
}

fn side_line(
    label: &str,
    side: Side,
    buy_label: &str,
    sell_label: &str,
    focused: bool,
) -> Line<'static> {
    let label_style = Style::default().add_modifier(Modifier::BOLD);
    let buy_style = if focused && side == Side::Buy {
        Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green)
    } else if side == Side::Buy {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let sell_style = if focused && side == Side::Sell {
        Style::default().add_modifier(Modifier::REVERSED).fg(Color::Red)
    } else if side == Side::Sell {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Line::from(vec![
        Span::styled(label.to_string(), label_style),
        Span::raw(" ["),
        Span::styled(buy_label.to_string(), buy_style),
        Span::raw("] ["),
        Span::styled(sell_label.to_string(), sell_style),
        Span::raw("]"),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ratatui::layout::Constraint::Percentage(percent_y),
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ratatui::layout::Constraint::Percentage(percent_x),
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match app.overlay {
        Some(Overlay::AddTransaction(_)) => handle_add_transaction_key(app, data, code).await,
        Some(Overlay::ConfirmDelete(_)) => handle_confirm_delete_key(app, data, code).await,
        None => KeyOutcome::Continue,
    }
}

async fn handle_add_transaction_key(
    app: &mut App,
    data: &mut UiData,
    code: KeyCode,
) -> KeyOutcome {
    let Some(Overlay::AddTransaction(form)) = app.overlay.as_mut() else {
        return KeyOutcome::Continue;
    };

    match code {
        KeyCode::Esc => {
            app.close_overlay();
            return KeyOutcome::Continue;
        }
        KeyCode::Tab | KeyCode::Down => {
            form.focused = form.focused.next();
            form.error = None;
            form.replace_on_input = form.focused != AddField::Side;
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.focused = form.focused.prev();
            form.error = None;
            form.replace_on_input = form.focused != AddField::Side;
        }
        KeyCode::Char(' ') if form.focused == AddField::Side => {
            form.toggle_side();
            form.error = None;
        }
        KeyCode::Enter => return submit_add_transaction(app, data).await,
        KeyCode::Backspace if form.focused != AddField::Side => {
            form.focused_mut().pop();
            form.error = None;
        }
        KeyCode::Char(c) if !c.is_control() && form.focused != AddField::Side => {
            if form.replace_on_input {
                form.focused_mut().clear();
                form.replace_on_input = false;
            }
            form.focused_mut().push(c);
            form.error = None;
        }
        _ => {}
    }
    KeyOutcome::Continue
}

async fn handle_confirm_delete_key(
    app: &mut App,
    data: &mut UiData,
    code: KeyCode,
) -> KeyOutcome {
    match code {
        KeyCode::Esc => app.close_overlay(),
        KeyCode::Enter | KeyCode::Char('y') => return confirm_delete(app, data).await,
        _ => {}
    }
    KeyOutcome::Continue
}

async fn submit_add_transaction(app: &mut App, data: &mut UiData) -> KeyOutcome {
    let b = app.bundle;
    let form = match app.overlay.as_ref() {
        Some(Overlay::AddTransaction(f)) => f.clone(),
        Some(Overlay::ConfirmDelete(_)) | None => return KeyOutcome::Continue,
    };

    let validated = match validate(&form) {
        Ok(v) => v,
        Err(kind) => {
            let msg = match kind {
                "symbol" => b.add_tx_err_symbol,
                "quantity" => b.add_tx_err_quantity,
                "price" => b.add_tx_err_price,
                "fees" => b.add_tx_err_fees,
                "date" => b.add_tx_err_date,
                _ => b.add_tx_err_submit,
            };
            if let Some(Overlay::AddTransaction(f)) = app.overlay.as_mut() {
                f.error = Some(msg.to_string());
            }
            return KeyOutcome::Continue;
        }
    };

    let side_str = match validated.side {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    };

    match client::add_transaction(
        &validated.symbol,
        side_str,
        &validated.quantity.to_string(),
        &validated.price.to_string(),
        &validated.fees.to_string(),
        &validated.executed_at.to_string(),
        validated.note.as_deref(),
    )
    .await
    {
        Ok(_) => {
            let sym = validated.symbol.clone();
            app.close_overlay();
            app.show_toast(Toast::info(format!("{}: {sym}", b.add_tx_success)));

            if let Ok(rows) = client::fetch_positions().await {
                data.positions = rows;
            }
            if data.detail.as_ref().is_some_and(|d| d.symbol == sym) {
                if let Ok(d) = client::fetch_detail(&sym).await {
                    data.detail = Some(d);
                }
            }
            data.ledger.clear();
        }
        Err(e) => {
            if let Some(Overlay::AddTransaction(f)) = app.overlay.as_mut() {
                f.error = Some(format!("{}: {e}", b.add_tx_err_submit));
            }
        }
    }
    KeyOutcome::Continue
}

async fn confirm_delete(app: &mut App, data: &mut UiData) -> KeyOutcome {
    let b = app.bundle;
    let (id, symbol) = match app.overlay.as_ref() {
        Some(Overlay::ConfirmDelete(d)) => (d.row.id, d.row.symbol.clone()),
        Some(Overlay::AddTransaction(_)) | None => return KeyOutcome::Continue,
    };

    match client::delete_transaction(id).await {
        Ok(true) => {
            app.close_overlay();
            app.show_toast(Toast::info(format!("{}: #{id}", b.delete_confirm_success)));

            if let Ok(rows) = client::fetch_ledger().await {
                data.ledger = rows;
                app.ledger_selected = app.ledger_selected.min(data.ledger.len().saturating_sub(1));
            }
            if let Ok(positions) = client::fetch_positions().await {
                data.positions = positions;
            }
            if data.detail.as_ref().is_some_and(|d| d.symbol == symbol) {
                match client::fetch_detail(&symbol).await {
                    Ok(d) => data.detail = Some(d),
                    Err(_) => data.detail = None,
                }
            }
        }
        Ok(false) => {
            if let Some(Overlay::ConfirmDelete(d)) = app.overlay.as_mut() {
                d.error = Some(b.delete_confirm_not_found.to_string());
            }
        }
        Err(e) => {
            if let Some(Overlay::ConfirmDelete(d)) = app.overlay.as_mut() {
                d.error = Some(format!("{}: {e}", b.delete_confirm_err));
            }
        }
    }
    KeyOutcome::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locale;
    use crate::core::types::Side;
    use crate::tui::app::App;
    use crate::tui::models::LedgerRow;
    use crate::tui::overlays::confirm_delete::ConfirmDelete;
    use ratatui::{backend::TestBackend, Terminal};
    use rust_decimal_macros::dec;

    #[test]
    fn renders_add_transaction_modal() {
        let mut app = App::new(Locale::PtBr);
        app.open_add_transaction(Some("PETR4".into()), None);
        let backend = TestBackend::new(100, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, f.area(), &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("PETR4"));
        assert!(text.contains("Nova transação"));
    }

    #[test]
    fn renders_confirm_delete_modal() {
        let mut app = App::new(Locale::PtBr);
        app.open_confirm_delete(ConfirmDelete::new(LedgerRow {
            id: 7,
            symbol: "PETR4".into(),
            side: Side::Buy,
            quantity: dec!(100),
            price: dec!(28.5),
            executed_at: "2026-01-02".into(),
        }));
        let backend = TestBackend::new(100, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, f.area(), &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("PETR4"));
        assert!(text.contains("Excluir transação"));
    }
}
