pub mod app;
pub mod client;
pub mod screens;
pub mod views;
pub mod widgets;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::Terminal;
use std::time::{Duration, Instant};

use app::{App, Screen, Toast};
use views::{DetailData, LedgerRow, PositionRow, SearchResultRow};
use widgets::status_bar;

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

struct UiData {
    positions: Vec<PositionRow>,
    detail: Option<DetailData>,
    search_results: Vec<SearchResultRow>,
    ledger: Vec<LedgerRow>,
}

impl UiData {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            detail: None,
            search_results: Vec::new(),
            ledger: Vec::new(),
        }
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

            match app.screen {
                Screen::Portfolio => views::render_portfolio(
                    f,
                    chunks[1],
                    app.bundle,
                    app.fmt,
                    &data.positions,
                    app.portfolio_selected,
                ),
                Screen::Detail => {
                    if let Some(detail) = &data.detail {
                        views::render_detail(f, chunks[1], app.bundle, app.fmt, detail);
                    }
                }
                Screen::Search => views::render_search(
                    f,
                    chunks[1],
                    app.bundle,
                    &app.search_query,
                    &data.search_results,
                    app.search_selected,
                ),
                Screen::Ledger => views::render_ledger(
                    f,
                    chunks[1],
                    app.bundle,
                    app.fmt,
                    &data.ledger,
                    app.ledger_selected,
                ),
            }

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
                match handle_key(&mut app, &mut data, k.code).await {
                    KeyOutcome::Quit => break Ok(()),
                    KeyOutcome::Continue => {}
                }
            }
        }

        // Debounced search while on Search screen.
        if app.screen == Screen::Search
            && app.search_pending
            && app
                .search_deadline
                .is_some_and(|d| Instant::now() >= d)
        {
            app.search_pending = false;
            let query = app.search_query.clone();
            if query.is_empty() {
                data.search_results.clear();
            } else {
                match client::search_assets(&query).await {
                    Ok(mut results) => {
                        client::mark_portfolio_hits(&mut results, &data.positions);
                        data.search_results = results;
                        if app.search_selected >= data.search_results.len() {
                            app.search_selected = data.search_results.len().saturating_sub(1);
                        }
                    }
                    Err(e) => {
                        data.search_results.clear();
                        app.show_toast(Toast::error(format!("{}: {e}", app.bundle.err_search)));
                    }
                }
            }
        }
    };

    res
}

enum KeyOutcome {
    Continue,
    Quit,
}

async fn handle_key(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Char('q') => return KeyOutcome::Quit,

        // Global navigation
        KeyCode::Char('1') => {
            app.go_portfolio();
            return KeyOutcome::Continue;
        }
        KeyCode::Char('2') => {
            app.go_search();
            return KeyOutcome::Continue;
        }
        KeyCode::Char('3') => {
            app.go_ledger();
            if data.ledger.is_empty() {
                match client::fetch_ledger().await {
                    Ok(rows) => data.ledger = rows,
                    Err(e) => app.show_toast(Toast::error(format!(
                        "{}: {e}",
                        app.bundle.err_fetch_ledger
                    ))),
                }
            }
            return KeyOutcome::Continue;
        }
        KeyCode::Char('/') => {
            app.go_search();
            return KeyOutcome::Continue;
        }

        _ => {}
    }

    match app.screen {
        Screen::Portfolio => handle_portfolio_keys(app, data, code).await,
        Screen::Detail => handle_detail_keys(app, data, code).await,
        Screen::Search => handle_search_keys(app, data, code).await,
        Screen::Ledger => handle_ledger_keys(app, data, code),
    }
}

async fn handle_portfolio_keys(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
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
        KeyCode::Char('o') => app.sort_by_score = !app.sort_by_score,
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

async fn handle_detail_keys(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
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
        _ => {}
    }
    KeyOutcome::Continue
}

async fn handle_search_keys(app: &mut App, data: &mut UiData, code: KeyCode) -> KeyOutcome {
    match code {
        KeyCode::Esc => {
            app.go_portfolio();
            app.search_query.clear();
            data.search_results.clear();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            data.search_results.clear();
            app.search_selected = 0;
            app.schedule_search(Duration::from_millis(400));
        }
        KeyCode::Char(c) if !c.is_control() => {
            app.search_query.push(c);
            data.search_results.clear();
            app.search_selected = 0;
            app.schedule_search(Duration::from_millis(400));
        }
        KeyCode::Down => {
            if !data.search_results.is_empty() {
                app.search_selected = (app.search_selected + 1).min(data.search_results.len() - 1);
            }
        }
        KeyCode::Up => {
            app.search_selected = app.search_selected.saturating_sub(1);
        }
        KeyCode::Enter => {
            // Phase 2: open Add Transaction modal with selected symbol.
            if let Some(row) = data.search_results.get(app.search_selected) {
                app.show_toast(Toast::info(format!(
                    "{} — add trade (phase 2)",
                    row.symbol
                )));
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

fn handle_ledger_keys(app: &mut App, data: &UiData, code: KeyCode) -> KeyOutcome {
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
