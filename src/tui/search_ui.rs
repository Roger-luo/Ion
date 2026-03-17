use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use ion_skill::search::skill_dir_name;

use super::search_app::{ListRow, SearchApp};
use super::util::wrap_text;

const LABEL_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const VALUE_STYLE: Style = Style::new().fg(Color::White);
const DIM_STYLE: Style = Style::new().fg(Color::DarkGray);

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
                let stars_str = format_stars_compact(*stars);
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

                let display_name: &str = if *grouped {
                    skill_dir_name(&r.source)
                } else {
                    &r.name
                };

                let stars_str = if *grouped {
                    String::new()
                } else {
                    format_stars_compact(r.stars)
                };

                let badge = if *grouped {
                    String::new()
                } else {
                    format!(" [{}]", r.registry)
                };

                let badge_color = registry_color(&r.registry);

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
    let block = Block::default().borders(Borders::ALL).title(" Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(row) = app.rows.get(app.selected) else {
        let msg = Paragraph::new("No result selected.").style(Style::default().fg(Color::DarkGray));
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
            registry,
        } => render_repo_detail(frame, inner, owner_repo, *stars, description, *skill_count, registry),
    }
}

fn render_skill_detail(frame: &mut Frame, app: &SearchApp, area: Rect, idx: usize) {
    let Some(r) = app.results.get(idx) else {
        return;
    };

    let owner = r
        .source
        .split_once('/')
        .map_or(r.source.as_str(), |(owner, _)| owner);
    let stars_str = r.stars.map_or("—".to_string(), |n| n.to_string());
    let wrap_width = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Source: ", LABEL_STYLE),
            Span::styled(
                registry_label(&r.registry),
                Style::new().fg(registry_color(&r.registry)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Owner: ", LABEL_STYLE),
            Span::styled(owner, VALUE_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Stars: ", LABEL_STYLE),
            Span::styled(format!("* {stars_str}"), VALUE_STYLE),
        ]),
        Line::from(""),
    ];

    push_wrapped_section(&mut lines, "Description:", &r.description, wrap_width);

    if let Some(ref skill_desc) = r.skill_description {
        push_wrapped_section(&mut lines, "Skill Description:", skill_desc, wrap_width);
    }

    if !r.source.is_empty() {
        lines.push(Line::from(Span::styled("Install:", LABEL_STYLE)));
        lines.push(Line::from(Span::styled(
            format!("  ion add {}", r.source),
            DIM_STYLE,
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
    registry: &str,
) {
    let owner = owner_repo
        .split_once('/')
        .map_or(owner_repo, |(owner, _)| owner);
    let stars_str = stars.map_or("—".to_string(), |n| n.to_string());
    let wrap_width = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Source: ", LABEL_STYLE),
            Span::styled(
                registry_label(registry),
                Style::new().fg(registry_color(registry)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", LABEL_STYLE),
            Span::styled(owner_repo, VALUE_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Owner: ", LABEL_STYLE),
            Span::styled(owner, VALUE_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Stars: ", LABEL_STYLE),
            Span::styled(format!("* {stars_str}"), VALUE_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Skills: ", LABEL_STYLE),
            Span::styled(skill_count.to_string(), VALUE_STYLE),
        ]),
        Line::from(""),
    ];

    push_wrapped_section(&mut lines, "Description:", description, wrap_width);

    lines.push(Line::from(Span::styled("Install all:", LABEL_STYLE)));
    lines.push(Line::from(Span::styled(
        format!("  ion add {owner_repo}"),
        DIM_STYLE,
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Append a labeled wrapped-text section to `lines` if text is non-empty.
fn push_wrapped_section<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &'a str,
    text: &str,
    wrap_width: usize,
) {
    if text.is_empty() {
        return;
    }
    lines.push(Line::from(Span::styled(label, LABEL_STYLE)));
    for wrapped_line in wrap_text(text, wrap_width) {
        lines.push(Line::from(Span::styled(
            format!("  {wrapped_line}"),
            VALUE_STYLE,
        )));
    }
    lines.push(Line::from(""));
}

fn format_stars_compact(stars: Option<u64>) -> String {
    match stars {
        Some(n) => format!(" *{n}"),
        None => String::new(),
    }
}

fn registry_label(registry: &str) -> &str {
    match registry {
        "github" => "GitHub",
        "skills.sh" | "skills-sh" => "skills.sh",
        "agent" => "Agent",
        "http" => "HTTP",
        other => other,
    }
}

fn registry_color(registry: &str) -> Color {
    match registry {
        "github" => Color::White,
        "agent" => Color::Magenta,
        "http" => Color::Green,
        _ => Color::Blue,
    }
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(Span::styled(
        " ↑↓/jk Navigate  Enter Install  q/Esc Quit",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}
