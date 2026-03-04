use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use super::app::{App, InputMode, Tab};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .split(area);

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
    render_help(frame, app, chunks[3]);
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
