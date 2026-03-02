use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::search_app::SearchApp;

pub fn handle_search_key(app: &mut SearchApp, key: KeyEvent) -> anyhow::Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            if !app.rows.is_empty() {
                app.should_install = true;
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }

    Ok(())
}
