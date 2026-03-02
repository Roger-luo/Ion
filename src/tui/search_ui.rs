use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use ion_skill::search::owner_repo_of;

use super::search_app::{ListRow, SearchApp};
use super::util::wrap_text;

pub fn render_search(frame: &mut Frame, app: &mut SearchApp) {
    let area = frame.area();

    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);

    let columns = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

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

    if app.rows.is_empty() {
        let msg =
            Paragraph::new("No installable results.").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let start = app.scroll_offset;
    let end = (start + app.visible_height).min(app.rows.len());

    let mut lines: Vec<Line> = Vec::new();
    for i in start..end {
        let row = &app.rows[i];
        let is_selected = i == app.selected;

        match row {
            ListRow::RepoHeader {
                owner_repo,
                stars,
                skill_count,
                ..
            } => {
                let prefix = if is_selected { "> " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                };
                let stars_str = match stars {
                    Some(n) => format!(" *{n}"),
                    None => String::new(),
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(owner_repo.as_str(), style),
                    Span::styled(
                        stars_str,
                        if is_selected {
                            style
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    Span::styled(
                        format!(" [{skill_count} skills]"),
                        Style::default().fg(Color::Blue),
                    ),
                ]));
            }
            ListRow::Skill {
                result_idx,
                grouped,
            } => {
                let r = &app.results[*result_idx];
                let indent = if *grouped { "    " } else { "" };
                let prefix = if is_selected { "> " } else { "  " };

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let stars_style = if is_selected {
                    style
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                // For grouped skills, show just the skill dir name; for ungrouped, show full name
                let display_name: String = if *grouped {
                    let owner_repo = owner_repo_of(&r.source);
                    r.source
                        .strip_prefix(owner_repo)
                        .and_then(|s| s.strip_prefix('/'))
                        .map(|s| s.rsplit('/').next().unwrap_or(s))
                        .unwrap_or(&r.name)
                        .to_string()
                } else {
                    r.name.clone()
                };

                let stars_str = if *grouped {
                    String::new() // stars shown on the header
                } else {
                    match r.stars {
                        Some(n) => format!(" *{n}"),
                        None => String::new(),
                    }
                };

                let badge = if *grouped {
                    String::new()
                } else {
                    let badge_color = match r.registry.as_str() {
                        "github" => Color::White,
                        "agent" => Color::Magenta,
                        _ => Color::Blue,
                    };
                    // We'll push this as a separate span below
                    let _ = badge_color;
                    format!(" [{}]", r.registry)
                };

                let badge_color = match r.registry.as_str() {
                    "github" => Color::White,
                    "agent" => Color::Magenta,
                    _ => Color::Blue,
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{indent}{prefix}"), style),
                    Span::styled(display_name, style),
                    Span::styled(stars_str, stars_style),
                    Span::styled(badge, Style::default().fg(badge_color)),
                ]));
            }
        }
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

    let Some(row) = app.rows.get(app.selected) else {
        let msg =
            Paragraph::new("No result selected.").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    };

    match row {
        ListRow::Skill { result_idx, .. } => render_skill_detail(frame, app, inner, *result_idx),
        ListRow::RepoHeader {
            owner_repo,
            stars,
            description,
            skill_count,
        } => render_repo_detail(frame, inner, owner_repo, *stars, description, *skill_count),
    }
}

fn render_skill_detail(frame: &mut Frame, app: &SearchApp, area: Rect, idx: usize) {
    let Some(r) = app.results.get(idx) else {
        return;
    };

    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let owner = r
        .name
        .split_once('/')
        .map_or(r.name.as_str(), |(owner, _)| owner);
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
        let wrap_width = area.width.saturating_sub(2) as usize;
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
        let wrap_width = area.width.saturating_sub(2) as usize;
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
    frame.render_widget(paragraph, area);
}

fn render_repo_detail(
    frame: &mut Frame,
    area: Rect,
    owner_repo: &str,
    stars: Option<u64>,
    description: &str,
    skill_count: usize,
) {
    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let owner = owner_repo
        .split_once('/')
        .map_or(owner_repo, |(owner, _)| owner);
    let stars_str = stars.map_or("—".to_string(), |n| n.to_string());

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Repo: ", label_style),
            Span::styled(owner_repo, value_style),
        ]),
        Line::from(vec![
            Span::styled("Owner: ", label_style),
            Span::styled(owner, value_style),
        ]),
        Line::from(vec![
            Span::styled("Stars: ", label_style),
            Span::styled(format!("* {stars_str}"), value_style),
        ]),
        Line::from(vec![
            Span::styled("Skills: ", label_style),
            Span::styled(skill_count.to_string(), value_style),
        ]),
        Line::from(""),
    ];

    if !description.is_empty() {
        lines.push(Line::from(Span::styled("Description:", label_style)));
        let wrap_width = area.width.saturating_sub(2) as usize;
        for wrapped_line in wrap_text(description, wrap_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped_line}"),
                value_style,
            )));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled("Install all:", label_style)));
    lines.push(Line::from(Span::styled(
        format!("  ion add {owner_repo}"),
        dim_style,
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(Span::styled(
        " ↑↓/jk Navigate  Enter Install  q/Esc Quit",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}
