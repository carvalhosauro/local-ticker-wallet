use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::core::format;
use crate::tui::app::{App, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::{score_color, DetailData};
use crate::tui::state::UiData;

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, detail: &DetailData) {
    let bundle = app.bundle;
    let fmt = app.fmt;
    let label = Style::default().add_modifier(Modifier::BOLD);
    let pnl = format!(
        "{} ({})",
        format::format_money(detail.unrealized_pnl, fmt),
        format::format_pct(detail.unrealized_pnl_pct, fmt)
    );
    let lines = vec![
        Line::from(vec![
            Span::styled(bundle.label_symbol, label),
            Span::raw(&detail.symbol),
        ]),
        Line::from(vec![
            Span::styled(bundle.label_quantity, label),
            Span::raw(format::format_quantity(detail.quantity, fmt)),
        ]),
        Line::from(vec![
            Span::styled(bundle.label_avg_cost, label),
            Span::raw(format::format_price(detail.avg_cost, fmt)),
        ]),
        Line::from(vec![
            Span::styled(bundle.label_market_value, label),
            Span::raw(format::format_money(detail.market_value, fmt)),
        ]),
        Line::from(vec![
            Span::styled(bundle.label_unrealized_pnl, label),
            Span::raw(pnl),
        ]),
        Line::from(vec![
            Span::styled(bundle.label_day_pct, label),
            Span::raw(format::format_pct(detail.day_change_pct, fmt)),
        ]),
        Line::from(""),
        Line::from(Span::styled(bundle.score_breakdown, label)),
        Line::from(format!(
            "  {}: {}",
            bundle.score_proximity_low,
            format::format_score_sub(detail.proximity_low, fmt)
        )),
        Line::from(format!(
            "  {}: {}",
            bundle.score_below_sma,
            format::format_score_sub(detail.below_sma, fmt)
        )),
        Line::from(format!(
            "  {}: {}",
            bundle.score_drawdown,
            format::format_score_sub(detail.drawdown, fmt)
        )),
        Line::from(format!(
            "  {}: {}",
            bundle.score_dividend_yield,
            format::format_score_sub(detail.dividend_yield, fmt)
        )),
        Line::from(format!(
            "  {}: {}",
            bundle.score_cost_vs_trend,
            format::format_score_sub(detail.cost_vs_trend, fmt)
        )),
        Line::from(vec![
            Span::styled(bundle.score_total, label),
            Span::styled(
                format::format_score(detail.total),
                Style::default().fg(score_color(detail.total)),
            ),
        ]),
    ];
    let title = format!("{} — {}", bundle.detail_title, bundle.detail_footer);
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(para, area);
}

pub async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => app.go_portfolio(),
        KeyCode::Char('r') => {
            if let Some(detail) = &data.detail {
                let sym = detail.symbol.clone();
                if let Err(e) = client::refresh_symbol(&sym).await {
                    app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_refresh)));
                } else {
                    match client::fetch_detail(&sym).await {
                        Ok(d) => data.detail = Some(d),
                        Err(e) => app.show_toast(Toast::error(format!(
                            "{}: {e}",
                            app.bundle.err_fetch_detail
                        ))),
                    }
                }
            }
        }
        KeyCode::Char('l') => {
            app.go_ledger();
            match client::fetch_ledger().await {
                Ok(rows) => data.ledger = rows,
                Err(e) => app.show_toast(Toast::error(format!(
                    "{}: {e}",
                    app.bundle.err_fetch_ledger
                ))),
            }
        }
        KeyCode::Char('a') => {
            if let Some(detail) = &data.detail {
                app.open_add_transaction(
                    Some(detail.symbol.clone()),
                    Some(detail.avg_cost.to_string()),
                );
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
    fn renders_detail_with_localized_labels() {
        let app = App::new(Locale::PtBr);
        let detail = DetailData {
            symbol: "VALE3".into(),
            quantity: dec!(200),
            avg_cost: dec!(60),
            market_value: dec!(13000),
            unrealized_pnl: dec!(1000),
            unrealized_pnl_pct: dec!(8.33),
            day_change_pct: dec!(-0.5),
            proximity_low: dec!(90),
            below_sma: dec!(50),
            drawdown: dec!(40),
            dividend_yield: dec!(70),
            cost_vs_trend: dec!(30),
            total: 61,
        };
        let text = buffer_text(100, 22, |f| render(f, f.area(), &app, &detail));
        assert!(text.contains("VALE3"));
        assert!(text.contains("Composição do score"));
        assert!(text.contains("61"));
    }
}
