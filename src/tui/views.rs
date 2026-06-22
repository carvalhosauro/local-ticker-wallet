use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

#[derive(Debug, Clone)]
pub struct PositionRow {
    pub symbol: String,
    pub quantity: String,
    pub avg_cost: String,
    pub market_value: String,
    pub day_change_pct: String,
    pub unrealized_pnl_pct: String,
    pub score: u8,
}

/// Full per-position view including the opportunity-score sub-scores.
#[derive(Debug, Clone)]
pub struct DetailData {
    pub symbol: String,
    pub quantity: String,
    pub avg_cost: String,
    pub market_value: String,
    pub unrealized_pnl: String,
    pub unrealized_pnl_pct: String,
    pub day_change_pct: String,
    pub proximity_low: String,
    pub below_sma: String,
    pub drawdown: String,
    pub dividend_yield: String,
    pub cost_vs_trend: String,
    pub total: u8,
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

pub fn render_positions(
    frame: &mut ratatui::Frame,
    area: Rect,
    rows: &[PositionRow],
    selected: usize,
) {
    let header = Row::new(["Symbol", "Qty", "Avg", "Mkt Value", "Day %", "P&L %", "Score"])
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
                Cell::from(r.quantity.clone()),
                Cell::from(r.avg_cost.clone()),
                Cell::from(r.market_value.clone()),
                Cell::from(r.day_change_pct.clone()),
                Cell::from(r.unrealized_pnl_pct.clone()),
                Cell::from(r.score.to_string()).style(Style::default().fg(score_color(r.score))),
            ])
            .style(style)
        })
        .collect();
    let widths = [Constraint::Length(10); 7];
    let table = Table::new(body, widths).header(header).block(
        Block::default().borders(Borders::ALL).title(
            "local-ticker-wallet — positions (q quit · r refresh · Enter detail)",
        ),
    );
    frame.render_widget(table, area);
}

/// Renders the detail view for a single position: core fields plus the five
/// opportunity-score sub-scores and the weighted total.
pub fn render_detail(frame: &mut ratatui::Frame, area: Rect, detail: &DetailData) {
    let label = Style::default().add_modifier(Modifier::BOLD);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Symbol: ", label),
            Span::raw(detail.symbol.clone()),
        ]),
        Line::from(vec![
            Span::styled("Quantity: ", label),
            Span::raw(detail.quantity.clone()),
        ]),
        Line::from(vec![
            Span::styled("Avg Cost: ", label),
            Span::raw(detail.avg_cost.clone()),
        ]),
        Line::from(vec![
            Span::styled("Market Value: ", label),
            Span::raw(detail.market_value.clone()),
        ]),
        Line::from(vec![
            Span::styled("Unrealized P&L: ", label),
            Span::raw(format!("{} ({}%)", detail.unrealized_pnl, detail.unrealized_pnl_pct)),
        ]),
        Line::from(vec![
            Span::styled("Day %: ", label),
            Span::raw(detail.day_change_pct.clone()),
        ]),
        Line::from(""),
        Line::from(Span::styled("Score breakdown", label)),
        Line::from(vec![
            Span::raw("  proximity_low: "),
            Span::raw(detail.proximity_low.clone()),
        ]),
        Line::from(vec![
            Span::raw("  below_sma:     "),
            Span::raw(detail.below_sma.clone()),
        ]),
        Line::from(vec![
            Span::raw("  drawdown:      "),
            Span::raw(detail.drawdown.clone()),
        ]),
        Line::from(vec![
            Span::raw("  dividend_yield:"),
            Span::raw(detail.dividend_yield.clone()),
        ]),
        Line::from(vec![
            Span::raw("  cost_vs_trend: "),
            Span::raw(detail.cost_vs_trend.clone()),
        ]),
    ];
    lines.push(Line::from(vec![
        Span::styled("  Total: ", label),
        Span::styled(
            detail.total.to_string(),
            Style::default().fg(score_color(detail.total)),
        ),
    ]));

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("position detail (Esc back · q quit)"),
    );
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

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
        let rows = vec![PositionRow {
            symbol: "PETR4".into(),
            quantity: "100".into(),
            avg_cost: "10.00".into(),
            market_value: "1200.00".into(),
            day_change_pct: "1.50".into(),
            unrealized_pnl_pct: "20.00".into(),
            score: 73,
        }];
        let text = buffer_text(80, 10, |f| render_positions(f, f.area(), &rows, 0));
        assert!(text.contains("PETR4"));
        assert!(text.contains("73"));
    }

    #[test]
    fn renders_detail_symbol_and_subscores() {
        let detail = DetailData {
            symbol: "VALE3".into(),
            quantity: "200".into(),
            avg_cost: "60.00".into(),
            market_value: "13000.00".into(),
            unrealized_pnl: "1000.00".into(),
            unrealized_pnl_pct: "8.33".into(),
            day_change_pct: "-0.50".into(),
            proximity_low: "90.0".into(),
            below_sma: "50.0".into(),
            drawdown: "40.0".into(),
            dividend_yield: "70.0".into(),
            cost_vs_trend: "30.0".into(),
            total: 61,
        };
        let text = buffer_text(80, 20, |f| render_detail(f, f.area(), &detail));
        assert!(text.contains("VALE3"), "detail must show the symbol");
        assert!(text.contains("proximity"), "detail must show a sub-score label");
        assert!(text.contains("Total"), "detail must show the total label");
        assert!(text.contains("61"), "detail must show the total value");
    }
}
