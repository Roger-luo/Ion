use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::search_app::SearchApp;

pub fn render_search(frame: &mut Frame, app: &mut SearchApp) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);

    let columns = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .split(chunks[0]);

    // Compute visible height from left column, accounting for borders (2 lines)
    let list_inner_height = columns[0].height.saturating_sub(2) as usize;
    app.visible_height = list_inner_height;

    render_list(frame, app, columns[0]);
    render_detail(frame, app, columns[1]);
    render_footer(frame, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &SearchApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search Results ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.results.is_empty() {
        let msg = Paragraph::new("No installable results.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let start = app.scroll_offset;
    let end = (start + app.visible_height).min(app.results.len());

    let mut lines: Vec<Line> = Vec::new();
    for i in start..end {
        let r = &app.results[i];
        let is_selected = i == app.selected;

        let prefix = if is_selected { "> " } else { "  " };

        let stars_str = match r.stars {
            Some(n) => format!(" *{n}"),
            None => String::new(),
        };

        let badge_color = match r.registry.as_str() {
            "github" => Color::White,
            "agent" => Color::Magenta,
            _ => Color::Blue,
        };

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let stars_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(&r.name, style),
            Span::styled(stars_str, stars_style),
            Span::styled(
                format!(" [{}]", r.registry),
                Style::default().fg(badge_color),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_detail(frame: &mut Frame, app: &SearchApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(r) = app.selected_result() else {
        let msg = Paragraph::new("No result selected.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    };

    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let owner = SearchApp::owner_of(r);
    let stars_str = r.stars.map_or("—".to_string(), |n| n.to_string());

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Owner: ", label_style),
            Span::styled(owner, value_style),
        ]),
        Line::from(vec![
            Span::styled("Stars: ", label_style),
            Span::styled(format!("* {stars_str}"), value_style),
        ]),
        Line::from(""),
    ];

    if !r.description.is_empty() {
        lines.push(Line::from(Span::styled("Description:", label_style)));
        let wrap_width = inner.width.saturating_sub(2) as usize;
        for wrapped_line in wrap_text(&r.description, wrap_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped_line}"),
                value_style,
            )));
        }
        lines.push(Line::from(""));
    }

    if let Some(ref skill_desc) = r.skill_description {
        lines.push(Line::from(Span::styled("Skill Description:", label_style)));
        let wrap_width = inner.width.saturating_sub(2) as usize;
        for wrapped_line in wrap_text(skill_desc, wrap_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped_line}"),
                value_style,
            )));
        }
        lines.push(Line::from(""));
    }

    if !r.source.is_empty() {
        lines.push(Line::from(Span::styled("Install:", label_style)));
        lines.push(Line::from(Span::styled(
            format!("  ion add {}", r.source),
            dim_style,
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(Span::styled(
        " ↑↓/jk Navigate  Enter Install  q/Esc Quit",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
