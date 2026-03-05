use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};

/// Known agent tool targets: (name, config_dir, default_skills_path).
const KNOWN_TARGETS: &[(&str, &str, &str)] = &[
    ("claude", ".claude", ".claude/skills"),
    ("cursor", ".cursor", ".cursor/skills"),
    ("windsurf", ".windsurf", ".windsurf/skills"),
];

/// Total lines the inline widget occupies:
/// 1 header + known targets + 1 custom row + 1 footer = KNOWN_TARGETS.len() + 3
const WIDGET_HEIGHT: u16 = KNOWN_TARGETS.len() as u16 + 3;

/// An item in the multi-select list.
#[cfg_attr(test, derive(Debug))]
struct SelectItem {
    name: String,
    path: String,
    selected: bool,
    detected: bool,
}

/// State for the init multi-select TUI.
#[cfg_attr(test, derive(Debug))]
struct InitSelect {
    items: Vec<SelectItem>,
    cursor: usize,
    custom_input: String,
    /// Byte-offset cursor within custom_input.
    input_cursor: usize,
    should_quit: bool,
    should_confirm: bool,
}

impl InitSelect {
    fn new(project_dir: &Path) -> Self {
        let detected: Vec<&str> = KNOWN_TARGETS
            .iter()
            .filter(|(_, dir, _)| project_dir.join(dir).is_dir())
            .map(|(name, _, _)| *name)
            .collect();

        let items = KNOWN_TARGETS
            .iter()
            .map(|(name, _, path)| {
                let is_detected = detected.contains(name);
                SelectItem {
                    name: name.to_string(),
                    path: path.to_string(),
                    selected: is_detected,
                    detected: is_detected,
                }
            })
            .collect();

        InitSelect {
            items,
            cursor: 0,
            custom_input: String::new(),
            input_cursor: 0,
            should_quit: false,
            should_confirm: false,
        }
    }

    fn on_custom_row(&self) -> bool {
        self.cursor == self.items.len()
    }

    fn total_rows(&self) -> usize {
        self.items.len() + 1 // +1 for custom input row
    }

    fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.cursor + 1 < self.total_rows() {
            self.cursor += 1;
        }
    }

    fn toggle(&mut self) {
        if !self.on_custom_row() {
            self.items[self.cursor].selected = !self.items[self.cursor].selected;
        }
    }

    fn into_targets(self) -> BTreeMap<String, String> {
        let mut targets = BTreeMap::new();
        for item in &self.items {
            if item.selected {
                targets.insert(item.name.clone(), item.path.clone());
            }
        }
        // Parse custom input: comma-separated "name:path" entries
        for entry in self.custom_input.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if let Some((name, path)) = entry.split_once(':') {
                targets.insert(name.trim().to_string(), path.trim().to_string());
            }
        }
        targets
    }
}

fn handle_key(app: &mut InitSelect, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if app.on_custom_row() {
        match key.code {
            KeyCode::Up => app.move_up(),
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Enter => app.should_confirm = true,
            KeyCode::Left => {
                if app.input_cursor > 0 {
                    // Step back to previous char boundary
                    app.input_cursor = app.custom_input[..app.input_cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                }
            }
            KeyCode::Right => {
                if app.input_cursor < app.custom_input.len() {
                    // Step forward past current char
                    app.input_cursor += app.custom_input[app.input_cursor..]
                        .chars()
                        .next()
                        .map_or(0, |c| c.len_utf8());
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if app.input_cursor > 0 {
                    let prev = app.custom_input[..app.input_cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                    app.custom_input.drain(prev..app.input_cursor);
                    app.input_cursor = prev;
                }
            }
            KeyCode::Char(c) => {
                app.custom_input.insert(app.input_cursor, c);
                app.input_cursor += c.len_utf8();
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Char(' ') => app.toggle(),
            KeyCode::Enter => app.should_confirm = true,
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            _ => {}
        }
    }
}

fn render(frame: &mut ratatui::Frame, app: &InitSelect) {
    let lines = build_lines(app);
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, frame.area());
}

fn build_lines<'a>(app: &'a InitSelect) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled(
            "Select targets",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " (↑↓ navigate, space toggle, enter confirm)",
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Known target rows
    for (i, item) in app.items.iter().enumerate() {
        let is_cursor = i == app.cursor;
        let checkbox = if item.selected { "[x]" } else { "[ ]" };
        let prefix = if is_cursor { "❯ " } else { "  " };

        let style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if item.selected {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut spans = vec![
            Span::styled(prefix, style),
            Span::styled(format!("{checkbox} "), style),
            Span::styled(&item.name, style),
            Span::styled(
                format!("  {}", item.path),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        if item.detected {
            spans.push(Span::styled(
                "  (detected)",
                Style::default().fg(Color::Green),
            ));
        }

        lines.push(Line::from(spans));
    }

    // Custom input row — part of the selectable list
    let is_cursor = app.on_custom_row();
    let prefix = if is_cursor { "❯ " } else { "  " };
    let style = if is_cursor {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if app.custom_input.is_empty() {
        if is_cursor {
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled("[ ] ", style),
                Span::styled("█", Style::default().fg(Color::Yellow)),
                Span::styled(" type name:path", Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled("[ ] custom target…", style),
            ]));
        }
    } else {
        let text_style = Style::default().fg(Color::White);
        let cursor_style = Style::default().fg(Color::Black).bg(Color::Yellow);

        let (before, after) = app.custom_input.split_at(app.input_cursor);
        let mut spans = vec![
            Span::styled(prefix, style),
            Span::styled("[x] ", style),
            Span::styled(before, text_style),
        ];

        if is_cursor {
            if let Some(ch) = after.chars().next() {
                // Highlight the char under the cursor
                let ch_len = ch.len_utf8();
                spans.push(Span::styled(&after[..ch_len], cursor_style));
                spans.push(Span::styled(&after[ch_len..], text_style));
            } else {
                // Cursor at end — show block after text
                spans.push(Span::styled(" ", cursor_style));
            }
        } else {
            spans.push(Span::styled(after, text_style));
        }

        lines.push(Line::from(spans));
    }

    // Footer
    let help_text = if app.on_custom_row() {
        "←→ move cursor  ↑ back  enter confirm  esc cancel"
    } else {
        "↑↓/jk move  space select  enter confirm  q/esc cancel"
    };
    lines.push(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

/// Run the interactive multi-select inline in the terminal.
/// Returns `Ok(None)` if the user cancelled, or `Ok(Some(targets))` on confirm.
pub fn run_init_select(project_dir: &Path) -> anyhow::Result<Option<BTreeMap<String, String>>> {
    let mut app = InitSelect::new(project_dir);

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let options = TerminalOptions {
        viewport: Viewport::Inline(WIDGET_HEIGHT),
    };
    let mut terminal = Terminal::with_options(backend, options)?;

    let result = (|| {
        loop {
            terminal.draw(|frame| render(frame, &app))?;

            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);
            }

            if app.should_quit {
                return Ok(None);
            }
            if app.should_confirm {
                return Ok(Some(app.into_targets()));
            }
        }
    })();

    disable_raw_mode()?;
    // Move cursor below the inline viewport so subsequent output doesn't overwrite it
    let pos = terminal.get_cursor_position()?;
    crossterm::execute!(io::stdout(), crossterm::cursor::MoveTo(0, pos.y + 1))?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_c() -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_app() -> InitSelect {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        // We need the tempdir to live long enough, but InitSelect only reads
        // directory contents in new(), so we can construct and return.
        let app = InitSelect::new(dir.path());
        // Verify detected state
        assert!(
            app.items[0].selected,
            "claude should be pre-selected (detected)"
        );
        assert!(!app.items[1].selected, "cursor should not be pre-selected");
        assert!(
            !app.items[2].selected,
            "windsurf should not be pre-selected"
        );
        app
    }

    // --- Navigation ---

    #[test]
    fn navigate_down_and_up() {
        let mut app = make_app();
        assert_eq!(app.cursor, 0);

        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 1);

        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 2);

        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 3); // custom row
        assert!(app.on_custom_row());

        // Can't go past the end
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 3);

        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.cursor, 2);
        assert!(!app.on_custom_row());
    }

    #[test]
    fn navigate_with_jk() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.cursor, 1);
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn cannot_navigate_above_top() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.cursor, 0);
    }

    // --- Toggle ---

    #[test]
    fn toggle_selection() {
        let mut app = make_app();
        // claude starts selected (detected)
        assert!(app.items[0].selected);
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(!app.items[0].selected);
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.items[0].selected);
    }

    #[test]
    fn toggle_undetected_item() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Down)); // cursor
        assert!(!app.items[1].selected);
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.items[1].selected);
    }

    #[test]
    fn toggle_does_nothing_on_custom_row() {
        let mut app = make_app();
        // Navigate to custom row
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        assert!(app.on_custom_row());
        // Space on custom row types a space, doesn't toggle
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert_eq!(app.custom_input, " ");
    }

    // --- Text input ---

    #[test]
    fn type_custom_input() {
        let mut app = make_app();
        // Navigate to custom row
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "myapp:path/to/skills".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        assert_eq!(app.custom_input, "myapp:path/to/skills");
        assert_eq!(app.input_cursor, app.custom_input.len());
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        assert_eq!(app.custom_input, "abc");

        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.custom_input, "ab");
        assert_eq!(app.input_cursor, 2);

        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.custom_input, "a");
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.custom_input, "");
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn delete_key_also_works_as_backspace() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "xy".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handle_key(&mut app, key(KeyCode::Delete));
        assert_eq!(app.custom_input, "x");
    }

    // --- Arrow keys in text input ---

    #[test]
    fn left_right_arrow_moves_cursor() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "abcd".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        assert_eq!(app.input_cursor, 4);

        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 3);

        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 2);

        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.input_cursor, 3);
    }

    #[test]
    fn left_at_start_stays() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        handle_key(&mut app, key(KeyCode::Char('a')));
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 0);
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn right_at_end_stays() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.input_cursor, 1);
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn insert_at_middle() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "ac".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        // Move left once (before 'c')
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 1);

        // Insert 'b' between 'a' and 'c'
        handle_key(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.custom_input, "abc");
        assert_eq!(app.input_cursor, 2);
    }

    #[test]
    fn backspace_at_middle() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }

        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        // Move left once (cursor after 'b', before 'c')
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 2);

        // Backspace deletes 'b'
        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.custom_input, "ac");
        assert_eq!(app.input_cursor, 1);
    }

    // --- Confirm / Cancel ---

    #[test]
    fn enter_confirms() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.should_confirm);
    }

    #[test]
    fn esc_cancels() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn q_cancels_on_item_row() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn q_types_q_on_custom_row() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert_eq!(app.custom_input, "q");
    }

    #[test]
    fn ctrl_c_cancels_anywhere() {
        let mut app = make_app();
        handle_key(&mut app, ctrl_c());
        assert!(app.should_quit);

        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        handle_key(&mut app, ctrl_c());
        assert!(app.should_quit);
    }

    // --- into_targets ---

    #[test]
    fn into_targets_includes_selected_and_custom() {
        let mut app = make_app();
        // claude is already selected; also select cursor
        handle_key(&mut app, key(KeyCode::Down));
        handle_key(&mut app, key(KeyCode::Char(' ')));

        // Navigate to custom row and type a custom target
        handle_key(&mut app, key(KeyCode::Down));
        handle_key(&mut app, key(KeyCode::Down));
        for c in "myapp:custom/path".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }

        let targets = app.into_targets();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets["claude"], ".claude/skills");
        assert_eq!(targets["cursor"], ".cursor/skills");
        assert_eq!(targets["myapp"], "custom/path");
    }

    #[test]
    fn into_targets_empty_when_nothing_selected() {
        let mut app = make_app();
        // Deselect claude
        handle_key(&mut app, key(KeyCode::Char(' ')));
        let targets = app.into_targets();
        assert!(targets.is_empty());
    }

    #[test]
    fn into_targets_ignores_malformed_custom_input() {
        let mut app = make_app();
        // Deselect claude
        handle_key(&mut app, key(KeyCode::Char(' ')));
        // Navigate to custom row, type something without ':'
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        for c in "badformat".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        let targets = app.into_targets();
        assert!(targets.is_empty());
    }

    // --- Detection ---

    #[test]
    fn detected_dirs_are_preselected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        std::fs::create_dir(dir.path().join(".cursor")).unwrap();
        let app = InitSelect::new(dir.path());
        assert!(app.items[0].selected); // claude
        assert!(app.items[0].detected);
        assert!(app.items[1].selected); // cursor
        assert!(app.items[1].detected);
        assert!(!app.items[2].selected); // windsurf
        assert!(!app.items[2].detected);
    }

    #[test]
    fn no_dirs_means_nothing_preselected() {
        let dir = tempfile::tempdir().unwrap();
        let app = InitSelect::new(dir.path());
        assert!(!app.items[0].selected);
        assert!(!app.items[1].selected);
        assert!(!app.items[2].selected);
    }

    // --- Up arrow on custom row returns to item list ---

    #[test]
    fn up_from_custom_row_goes_to_last_item() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        assert!(app.on_custom_row());
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.cursor, app.items.len() - 1);
        assert!(!app.on_custom_row());
    }

    // --- Render: cursor display ---
    // Lines: 0=header, 1=claude, 2=cursor, 3=windsurf, 4=custom, 5=footer

    const CUSTOM_ROW: usize = 4;

    /// Helper: collect all span content from a Line into a single string.
    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// Helper: find the span that has a background color set (the cursor highlight).
    fn find_bg_span<'a>(line: &'a Line) -> Option<&'a Span<'a>> {
        line.spans.iter().find(|s| s.style.bg.is_some())
    }

    #[test]
    fn render_empty_input_focused_shows_block_cursor() {
        let mut app = make_app();
        // Navigate to custom row
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        let lines = build_lines(&app);
        let text = line_text(&lines[CUSTOM_ROW]);
        assert!(
            text.contains('█'),
            "empty focused input should show block cursor"
        );
        assert!(
            text.contains("type name:path"),
            "should show placeholder hint"
        );
    }

    #[test]
    fn render_empty_input_unfocused_shows_placeholder() {
        let app = make_app(); // cursor at row 0, not custom row
        let lines = build_lines(&app);
        let text = line_text(&lines[CUSTOM_ROW]);
        assert!(
            text.contains("custom target…"),
            "unfocused should show placeholder"
        );
        assert!(
            !text.contains('█'),
            "unfocused should not show block cursor"
        );
    }

    #[test]
    fn render_cursor_at_end_highlights_trailing_space() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        // input_cursor == 3, at end
        let lines = build_lines(&app);
        let bg_span =
            find_bg_span(&lines[CUSTOM_ROW]).expect("should have a highlighted cursor span");
        assert_eq!(
            bg_span.content.as_ref(),
            " ",
            "cursor at end should highlight a space"
        );
    }

    #[test]
    fn render_cursor_at_middle_highlights_char_under_cursor() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        // Move cursor left to sit on 'c'
        handle_key(&mut app, key(KeyCode::Left));

        let lines = build_lines(&app);
        let bg_span =
            find_bg_span(&lines[CUSTOM_ROW]).expect("should have a highlighted cursor span");
        assert_eq!(
            bg_span.content.as_ref(),
            "c",
            "should highlight char under cursor"
        );
    }

    #[test]
    fn render_cursor_at_start_highlights_first_char() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        // Move cursor to start
        handle_key(&mut app, key(KeyCode::Left));
        handle_key(&mut app, key(KeyCode::Left));
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.input_cursor, 0);

        let lines = build_lines(&app);
        let bg_span =
            find_bg_span(&lines[CUSTOM_ROW]).expect("should have a highlighted cursor span");
        assert_eq!(bg_span.content.as_ref(), "a", "should highlight first char");
    }

    #[test]
    fn render_no_highlight_when_custom_row_unfocused() {
        let mut app = make_app();
        // Navigate to custom row, type text, then move back up
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        for c in "abc".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handle_key(&mut app, key(KeyCode::Up)); // back to windsurf row

        let lines = build_lines(&app);
        assert!(
            find_bg_span(&lines[CUSTOM_ROW]).is_none(),
            "unfocused custom row should not have cursor highlight"
        );
        // Text should still be present
        let text = line_text(&lines[CUSTOM_ROW]);
        assert!(text.contains("abc"));
    }

    #[test]
    fn render_footer_shows_arrow_hint_on_custom_row() {
        let mut app = make_app();
        for _ in 0..app.items.len() {
            handle_key(&mut app, key(KeyCode::Down));
        }
        let lines = build_lines(&app);
        let footer = line_text(lines.last().unwrap());
        assert!(
            footer.contains("←→"),
            "custom row footer should mention arrow keys"
        );
    }

    #[test]
    fn render_footer_shows_space_hint_on_item_row() {
        let app = make_app();
        let lines = build_lines(&app);
        let footer = line_text(lines.last().unwrap());
        assert!(
            footer.contains("space"),
            "item row footer should mention space to select"
        );
    }
}
