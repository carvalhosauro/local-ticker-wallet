pub mod app;
pub mod client;
pub mod input;
pub mod models;
pub mod overlays;
pub mod screens;
pub mod state;
pub mod widgets;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::Terminal;
use std::time::Duration;

use app::{App, Toast};
use input::KeyOutcome;
use state::UiData;
use widgets::status_bar;

fn render_ui(f: &mut ratatui::Frame, app: &App, data: &UiData, main_area: ratatui::layout::Rect) {
    screens::render(f, main_area, app, data);
    if app.has_overlay() {
        overlays::render(f, main_area, app);
    }
}

/// RAII guard that restores the terminal on drop (covers `?`-errors and panics).
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}

pub async fn run() -> anyhow::Result<()> {
    let cfg = crate::config::Config::load()?;
    let mut app = App::new(cfg.locale);
    let mut data = UiData::new();

    match client::fetch_positions().await {
        Ok(rows) => data.positions = rows,
        Err(e) => app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_fetch_positions))),
    }

    enable_raw_mode()?;
    let _guard = TermGuard;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = loop {
        client::sort_positions(&mut data.positions, app.sort_by_score);

        term.draw(|f| {
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(if app.toast.is_some() { 3 } else { 0 }),
            ])
            .split(f.area());

            status_bar::render_status_bar(f, chunks[0], &app);
            render_ui(f, &app, &data, chunks[1]);

            if app.toast.is_some() {
                status_bar::render_toast(f, chunks[2], &app);
            }
        })?;

        app.tick_toast();

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match screens::handle_key(&mut app, &mut data, k.code).await {
                    KeyOutcome::Quit => break Ok(()),
                    KeyOutcome::Continue => {}
                }
            }
        }

        screens::tick(&mut app, &mut data).await;
    };

    res
}
