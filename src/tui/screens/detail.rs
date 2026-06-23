use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::core::format::{self, FormatLocale};
use crate::core::types::AssetId;
use crate::tui::app::{App, Toast};
use crate::tui::client;
use crate::tui::input::KeyOutcome;
use crate::tui::models::{score_color, DetailData};
use crate::tui::state::UiData;
use crate::tui::widgets::braille_chart;

pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, detail: &DetailData) {
    let fmt = app.fmt;
    let label = Style::default().add_modifier(Modifier::BOLD);

    let chunks = Layout::vertical([
        Constraint::Min(12),
        Constraint::Length(8),
    ])
    .split(area);

    render_stats(frame, chunks[0], app, detail, label, fmt);
    render_chart(frame, chunks[1], app, detail);
}

fn render_stats(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    detail: &DetailData,
    label: Style,
    fmt: FormatLocale,
) {
    let bundle = app.bundle;
    let currency = AssetId::b3(&detail.symbol).currency();
    let pnl = format!(
        "{} ({})",
        format::format_money_for_currency(detail.unrealized_pnl, currency, fmt),
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
            Span::raw(format::format_money_for_currency(
                detail.market_value, currency, fmt,
            )),
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

fn render_chart(frame: &mut ratatui::Frame, area: Rect, app: &App, detail: &DetailData) {
    let bundle = app.bundle;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", bundle.chart_title));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if detail.chart_closes.is_empty() {
        let para = Paragraph::new(Line::from(Span::raw(bundle.chart_empty)));
        frame.render_widget(para, inner);
        return;
    }

    let chart_width = inner.width.max(1);
    let chart_height = inner.height.max(1);
    let lines = braille_chart::render_lines(&detail.chart_closes, chart_width, chart_height);
    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

pub fn render_unavailable(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let bundle = app.bundle;
    let para = Paragraph::new(bundle.detail_unavailable).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} — {} ", bundle.detail_title, bundle.detail_footer)),
    );
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
    use rust_decimal::Decimal;
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

    fn sample_detail() -> DetailData {
        DetailData {
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
            chart_closes: (0..40)
                .map(|i| dec!(55) + Decimal::from(i) / Decimal::from(5))
                .collect(),
        }
    }

    #[test]
    fn renders_detail_with_localized_labels() {
        let app = App::new(Locale::PtBr);
        let detail = sample_detail();
        let text = buffer_text(100, 28, |f| render(f, f.area(), &app, &detail));
        assert!(text.contains("VALE3"));
        assert!(text.contains("Composição do score"));
        assert!(text.contains("61"));
        assert!(text.contains("Histórico de preços"));
    }

    #[test]
    fn renders_chart_empty_message() {
        let app = App::new(Locale::En);
        let mut detail = sample_detail();
        detail.chart_closes.clear();
        let text = buffer_text(80, 24, |f| render(f, f.area(), &app, &detail));
        assert!(text.contains("No history"));
    }

    #[test]
    fn renders_detail_unavailable_message() {
        let app = App::new(Locale::En);
        let text = buffer_text(80, 16, |f| render_unavailable(f, f.area(), &app));
        assert!(text.contains("Failed to load detail"));
    }
}
