use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use super::app::{App, InputMode, Tab};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let hint_lines = hint_height(app);
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(hint_lines),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .split(area);

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_hint(frame, app, chunks[2]);
    render_status(frame, app, chunks[3]);
    render_help(frame, app, chunks[4]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Global", "Project"];
    let selected = match app.tab {
        Tab::Global => 0,
        Tab::Project => 1,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Ion Config "))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let sections = app.current_sections();

    if app.tab == Tab::Project && !app.has_project {
        let msg = Paragraph::new("No Ion.toml found in current directory.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(msg, area);
        return;
    }

    if sections.is_empty() || app.total_entries() == 0 {
        let msg = Paragraph::new("No config values set. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(msg, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_index = 0;

    for section in sections {
        lines.push(Line::from(Span::styled(
            format!("  [{}]", section.name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        for (key, value) in &section.entries {
            let is_selected = entry_index == app.cursor;
            let prefix = if is_selected { "  > " } else { "    " };
            let dots = ".".repeat(30usize.saturating_sub(key.len()));

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if value == "(unset)" {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{prefix}{key} "), style),
                Span::styled(format!("{dots} "), Style::default().fg(Color::DarkGray)),
                Span::styled(value.to_string(), value_style),
            ]));

            entry_index += 1;
        }

        lines.push(Line::from(""));
    }

    let content =
        Paragraph::new(lines).block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
    frame.render_widget(content, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.input_mode {
        InputMode::EditingValue => {
            let (key, _) = app.current_entry().unwrap_or_default();
            Line::from(vec![
                Span::styled("Edit value for ", Style::default().fg(Color::Yellow)),
                Span::styled(key, Style::default().fg(Color::Cyan)),
                Span::styled(": ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        }
        InputMode::AddingKey => Line::from(vec![
            Span::styled("New key: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        InputMode::AddingValue => Line::from(vec![
            Span::styled(
                format!("Value for {}: ", app.adding_key_buffer),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(&app.input_buffer),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        InputMode::ConfirmDelete => {
            let (key, _) = app.current_entry().unwrap_or_default();
            Line::from(Span::styled(
                format!("Delete '{key}'? (y/n)"),
                Style::default().fg(Color::Red),
            ))
        }
        InputMode::ConfirmQuit => Line::from(Span::styled(
            "Unsaved changes. Save before quitting? (y/n/Esc cancel)",
            Style::default().fg(Color::Red),
        )),
        InputMode::Normal => {
            if let Some(ref msg) = app.status_message {
                Line::from(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::Green),
                ))
            } else if app.dirty {
                Line::from(Span::styled(
                    " [unsaved changes]",
                    Style::default().fg(Color::Yellow),
                ))
            } else {
                Line::from("")
            }
        }
    };

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

/// Return (description, example) for the currently selected config entry.
fn hint_for_entry(app: &App) -> Option<(&'static str, &'static str)> {
    let (si, ei) = app.cursor_position()?;
    let section = &app.current_sections()[si];
    let key = section.entries[ei].0.as_str();

    match (app.tab, section.name.as_str(), key) {
        // Global targets
        (Tab::Global, "targets", "claude") => Some((
            "Directory where skills are deployed for Claude Code.",
            "e.g. .claude/skills",
        )),
        (Tab::Global, "targets", "cursor") => Some((
            "Directory where skills are deployed for Cursor.",
            "e.g. .cursor/rules",
        )),
        (Tab::Global, "targets", "windsurf") => Some((
            "Directory where skills are deployed for Windsurf.",
            "e.g. .windsurf/rules",
        )),
        (Tab::Global, "targets", _) => Some((
            "Directory where skills are deployed for this agent target.",
            "e.g. .agent/skills",
        )),
        // Global sources
        (Tab::Global, "sources", _) => Some((
            "A named search source. Value is a GitHub owner or owner/repo.",
            "e.g. obra/skills, anthropics",
        )),
        // Global cache
        (Tab::Global, "cache", "max-age-days") => Some((
            "How many days to cache search results before re-fetching.",
            "e.g. 1, 7",
        )),
        // Global UI
        (Tab::Global, "ui", "color") => Some((
            "Enable or disable colored output.",
            "e.g. true, false",
        )),
        // Project targets
        (Tab::Project, "targets", _) => Some((
            "Project-level override for this agent target directory.",
            "e.g. .claude/skills",
        )),
        // Project options
        (Tab::Project, "options", "skills-dir") => Some((
            "Directory where local skills are stored. Skills live at <skills-dir>/skills/<name>/.",
            "e.g. .agents, skills",
        )),
        _ => None,
    }
}

fn hint_height(app: &App) -> u16 {
    if app.input_mode != InputMode::Normal {
        return 0;
    }
    if hint_for_entry(app).is_some() { 2 } else { 0 }
}

fn render_hint(frame: &mut Frame, app: &App, area: Rect) {
    if app.input_mode != InputMode::Normal {
        return;
    }

    if let Some((desc, example)) = hint_for_entry(app) {
        let hint_style = Style::default().fg(Color::DarkGray);
        let lines = vec![
            Line::from(Span::styled(format!(" {desc}"), hint_style)),
            Line::from(Span::styled(format!(" {example}"), hint_style)),
        ];
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.input_mode {
        InputMode::EditingValue | InputMode::AddingKey | InputMode::AddingValue => {
            "Enter Confirm  Esc Cancel"
        }
        InputMode::ConfirmDelete => "y Confirm  n Cancel",
        InputMode::ConfirmQuit => "y Save & quit  n Quit without saving  Esc Cancel",
        InputMode::Normal => {
            "↑↓ Navigate  ←→ Tab  Enter Edit  a Add  d Delete  s Save  q Quit"
        }
    };

    let help = Paragraph::new(Line::from(Span::styled(
        format!(" {help_text}"),
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}
