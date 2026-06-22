use crate::core::format::{self, FormatLocale};
use crate::core::types::Side;
use crate::i18n::Bundle;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct PositionRow {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub day_change_pct: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub score: u8,
}

#[derive(Debug, Clone)]
pub struct DetailData {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_pct: Decimal,
    pub day_change_pct: Decimal,
    pub proximity_low: Decimal,
    pub below_sma: Decimal,
    pub drawdown: Decimal,
    pub dividend_yield: Decimal,
    pub cost_vs_trend: Decimal,
    pub total: u8,
}

#[derive(Debug, Clone)]
pub struct SearchResultRow {
    pub symbol: String,
    pub name: String,
    pub kind: String,
    pub currency: String,
    pub in_portfolio: bool,
}

#[derive(Debug, Clone)]
pub struct LedgerRow {
    pub id: i64,
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub executed_at: String,
}

pub fn score_color(score: u8) -> Color {
    if score >= 70 {
        Color::Green
    } else if score >= 40 {
        Color::Yellow
    } else {
        Color::Red
    }
}

pub fn render_portfolio(
    frame: &mut ratatui::Frame,
    area: Rect,
    bundle: &Bundle,
    fmt: FormatLocale,
    rows: &[PositionRow],
    selected: usize,
) {
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
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(r.symbol.clone()),
                Cell::from(format::format_quantity(r.quantity, fmt)),
                Cell::from(format::format_price(r.avg_cost, fmt)),
                Cell::from(format::format_money(r.market_value, fmt)),
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
    let title = format!("{} — {}", bundle.app_title, bundle.portfolio_footer);
    let table = Table::new(body, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(table, area);
}

pub fn render_detail(
    frame: &mut ratatui::Frame,
    area: Rect,
    bundle: &Bundle,
    fmt: FormatLocale,
    detail: &DetailData,
) {
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

pub fn render_search(
    frame: &mut ratatui::Frame,
    area: Rect,
    bundle: &Bundle,
    query: &str,
    results: &[SearchResultRow],
    selected: usize,
) {
    let chunks = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Min(3),
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
        let empty = Paragraph::new(bundle.search_no_results).block(Block::default().borders(Borders::ALL));
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

pub fn render_ledger(
    frame: &mut ratatui::Frame,
    area: Rect,
    bundle: &Bundle,
    fmt: FormatLocale,
    rows: &[LedgerRow],
    selected: usize,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locale;
    use crate::i18n::bundle;
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
        let b = bundle(Locale::En);
        let fmt = FormatLocale::from(Locale::En);
        let rows = vec![PositionRow {
            symbol: "PETR4".into(),
            quantity: dec!(100),
            avg_cost: dec!(10),
            market_value: dec!(1200),
            day_change_pct: dec!(1.5),
            unrealized_pnl_pct: dec!(20),
            score: 73,
        }];
        let text = buffer_text(100, 12, |f| {
            render_portfolio(f, f.area(), b, fmt, &rows, 0)
        });
        assert!(text.contains("PETR4"));
        assert!(text.contains("73"));
    }

    #[test]
    fn renders_detail_with_localized_labels() {
        let b = bundle(Locale::PtBr);
        let fmt = FormatLocale::from(Locale::PtBr);
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
        let text = buffer_text(100, 22, |f| render_detail(f, f.area(), b, fmt, &detail));
        assert!(text.contains("VALE3"));
        assert!(text.contains("Composição do score"));
        assert!(text.contains("61"));
    }
}
