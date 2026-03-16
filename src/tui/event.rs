use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, ConfigEntry, ConfigSection, InputMode, Tab};

pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match app.input_mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::EditingValue => handle_editing(app, key),
        InputMode::AddingKey => handle_adding_key(app, key),
        InputMode::AddingValue => handle_adding_value(app, key),
        InputMode::ConfirmDelete => handle_confirm_delete(app, key),
        InputMode::ConfirmQuit => handle_confirm_quit(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.dirty {
                app.input_mode = InputMode::ConfirmQuit;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Up => {
            if app.total_entries() > 0 && app.cursor > 0 {
                app.cursor -= 1;
            }
            app.status_message = None;
        }
        KeyCode::Down => {
            let max = app.total_entries().saturating_sub(1);
            if app.cursor < max {
                app.cursor += 1;
            }
            app.status_message = None;
        }
        KeyCode::Left => {
            app.tab = Tab::Global;
            app.cursor = 0;
            app.status_message = None;
        }
        KeyCode::Right => {
            app.tab = Tab::Project;
            app.cursor = 0;
            app.status_message = None;
        }
        KeyCode::Enter => {
            if app.total_entries() > 0
                && let Some((_, value)) = app.current_entry()
            {
                app.input_buffer = if app.current_entry_is_default() {
                    String::new()
                } else {
                    value
                };
                app.input_mode = InputMode::EditingValue;
            }
        }
        KeyCode::Char('a') => {
            if app.tab == Tab::Project && !app.has_project {
                app.status_message = Some("No Ion.toml in current directory.".to_string());
                return Ok(());
            }
            app.input_buffer.clear();
            app.input_mode = InputMode::AddingKey;
        }
        KeyCode::Char('d') => {
            if app.total_entries() > 0 {
                app.input_mode = InputMode::ConfirmDelete;
            }
        }
        KeyCode::Char('s') => {
            if app.dirty {
                app.save()?;
            } else {
                app.status_message = Some("No changes to save.".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_editing(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            if let Some((si, ei)) = app.cursor_position() {
                let new_value = app.input_buffer.clone();
                let entry = &mut app.current_sections_mut()[si].entries[ei];
                entry.value = new_value;
                entry.is_default = false;
                app.dirty = true;
                app.status_message = Some("Value updated.".to_string());
            }
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_adding_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            if app.input_buffer.is_empty() {
                app.input_mode = InputMode::Normal;
                return Ok(());
            }

            let (section_name, field_name) =
                if let Some((s, f)) = app.input_buffer.split_once('.') {
                    (s.to_string(), f.to_string())
                } else if let Some(name) = app.current_section_name() {
                    (name, app.input_buffer.clone())
                } else {
                    ("targets".to_string(), app.input_buffer.clone())
                };

            app.adding_key_buffer = format!("{section_name}.{field_name}");
            app.input_buffer.clear();
            app.input_mode = InputMode::AddingValue;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_adding_value(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            let full_key = app.adding_key_buffer.clone();
            let value = app.input_buffer.clone();

            if let Some((section_name, field_name)) = full_key.split_once('.') {
                let sections = app.current_sections_mut();
                let section = sections.iter_mut().find(|s| s.name == section_name);

                if let Some(section) = section {
                    section
                        .entries
                        .push(ConfigEntry::new(field_name, &value));
                } else {
                    sections.push(ConfigSection {
                        name: section_name.to_string(),
                        entries: vec![ConfigEntry::new(field_name, &value)],
                    });
                }
                app.dirty = true;
                app.status_message = Some(format!("Added {full_key}."));
            }

            app.input_buffer.clear();
            app.adding_key_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.adding_key_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') => {
            if let Some((si, ei)) = app.cursor_position() {
                let sections = app.current_sections_mut();
                let removed = sections[si].entries.remove(ei);

                if sections[si].entries.is_empty()
                    && sections[si].name != "cache"
                    && sections[si].name != "ui"
                {
                    sections.remove(si);
                }

                let total = app.total_entries();
                if total > 0 && app.cursor >= total {
                    app.cursor = total - 1;
                }

                app.dirty = true;
                app.status_message = Some(format!("Deleted '{}'.", removed.key));
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_quit(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') => {
            app.save()?;
            app.should_quit = true;
        }
        KeyCode::Char('n') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Ok(())
}
