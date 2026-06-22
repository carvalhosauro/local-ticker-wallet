use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::{App, Screen};

pub fn render_status_bar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let b = app.bundle;
    let tabs = [
        (Screen::Portfolio, "1", b.nav_portfolio),
        (Screen::Search, "2", b.nav_search),
        (Screen::Ledger, "3", b.nav_ledger),
    ];
    let mut spans: Vec<ratatui::text::Span> = Vec::new();
    spans.push(ratatui::text::Span::styled(
        format!(" {} ", b.app_title),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    for (screen, key, label) in tabs {
        let active = app.screen == screen
            || (app.screen == Screen::Detail && screen == Screen::Portfolio);
        let style = if active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(ratatui::text::Span::raw(" "));
        spans.push(ratatui::text::Span::styled(format!("[{key}]{label}"), style));
    }
    let block = Block::default().borders(Borders::BOTTOM);
    let para = Paragraph::new(ratatui::text::Line::from(spans)).block(block);
    frame.render_widget(para, area);
}

pub fn render_toast(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let Some(toast) = &app.toast else {
        return;
    };
    let color = if toast.is_error {
        Color::Red
    } else {
        Color::Green
    };
    let para = Paragraph::new(toast.message.as_str())
        .style(Style::default().fg(color))
        .block(Block::default().borders(Borders::ALL).title(" status "));
    frame.render_widget(para, area);
}
