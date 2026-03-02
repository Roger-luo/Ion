use std::io;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub type Term = Terminal<CrosstermBackend<io::Stdout>>;

/// Run a TUI application with proper terminal setup and cleanup.
/// The `body` closure receives a mutable reference to the terminal and runs the
/// main event loop. Terminal is always restored, even on error.
pub fn run_tui<F, T>(body: F) -> anyhow::Result<T>
where
    F: FnOnce(&mut Term) -> anyhow::Result<T>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = body(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
