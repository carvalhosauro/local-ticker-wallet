pub mod client;
pub mod views;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::time::Duration;
use views::DetailData;

/// Which screen the TUI is currently showing.
enum Mode {
    List,
    // Boxed: `DetailData` is far larger than the `List` variant; boxing keeps
    // `Mode` small (clippy::large_enum_variant).
    Detail(Box<DetailData>),
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
    let mut rows = client::fetch_positions().await.unwrap_or_default();
    let mut selected = 0usize;
    let mut mode = Mode::List;

    enable_raw_mode()?;
    // Guard is installed immediately after raw mode is enabled so that any
    // subsequent `?` or panic still triggers teardown via Drop.
    let _guard = TermGuard;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = loop {
        term.draw(|f| match &mode {
            Mode::List => views::render_positions(f, f.area(), &rows, selected),
            Mode::Detail(detail) => views::render_detail(f, f.area(), detail.as_ref()),
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                match (&mode, k.code) {
                    // Quit from anywhere.
                    (_, KeyCode::Char('q')) => break Ok(()),

                    // --- List mode ---
                    (Mode::List, KeyCode::Char('r')) => {
                        let _ = client::refresh_all().await;
                        rows = client::fetch_positions().await.unwrap_or_default();
                        if selected >= rows.len() {
                            selected = rows.len().saturating_sub(1);
                        }
                    }
                    (Mode::List, KeyCode::Down) => {
                        if !rows.is_empty() {
                            selected = (selected + 1).min(rows.len() - 1);
                        }
                    }
                    (Mode::List, KeyCode::Up) => {
                        selected = selected.saturating_sub(1);
                    }
                    (Mode::List, KeyCode::Enter) => {
                        if let Some(row) = rows.get(selected) {
                            if let Ok(detail) = client::fetch_detail(&row.symbol).await {
                                mode = Mode::Detail(Box::new(detail));
                            }
                        }
                    }

                    // --- Detail mode ---
                    (Mode::Detail(_), KeyCode::Esc) => {
                        mode = Mode::List;
                    }

                    _ => {}
                }
            }
        }
    };

    // `_guard` drops here (normal exit) and runs teardown.
    res
}
