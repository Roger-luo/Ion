use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use ion_skill::search::skill_dir_name;

use super::search_app::{Hyperlink, ListRow, SearchApp};
use super::util::wrap_text;

const LABEL_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const VALUE_STYLE: Style = Style::new().fg(Color::White);
const LINK_STYLE: Style = Style::new()
    .fg(Color::Blue)
    .add_modifier(Modifier::UNDERLINED);
const DIM_STYLE: Style = Style::new().fg(Color::DarkGray);

/// Build a web URL for a result source.
fn source_url(registry: &str, source: &str) -> String {
    match registry {
        "skills.sh" | "skills-sh" => format!("https://skills.sh/{source}"),
        _ => format!("https://github.com/{source}"),
    }
}

pub fn render_search(frame: &mut Frame, app: &mut SearchApp) {
    app.hyperlinks.clear();
    let area = frame.area();

    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);

    let columns = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    render_list(frame, app, columns[0]);
    render_detail(frame, app, columns[1]);
    render_footer(frame, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &mut SearchApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search Results ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.rows.is_empty() {
        app.visible_height = 0;
        let msg =
            Paragraph::new("No installable results.").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let max_lines = inner.height as usize;
    let max_desc_width = inner.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();
    let mut row_idx = app.scroll_offset;
    let mut rows_rendered: usize = 0;

    while row_idx < app.rows.len() && lines.len() < max_lines {
        let row = &app.rows[row_idx];
        let is_selected = row_idx == app.selected;

        match row {
            ListRow::RepoHeader {
                owner_repo,
                stars,
                weekly_installs,
                skill_count,
                description,
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
                let stars_str = format_metric_compact_raw(*stars, *weekly_installs);
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

                // Add description line if there's room
                if !description.is_empty() && lines.len() < max_lines {
                    let indent_width = 2;
                    let available = max_desc_width.saturating_sub(indent_width);
                    let truncated = truncate_str(description, available);
                    let desc_indent = " ".repeat(indent_width);
                    lines.push(Line::from(Span::styled(
                        format!("{desc_indent}{truncated}"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
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
                    format_metric_compact(r)
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

                // Add description line if there's room
                let desc = r
                    .skill_description
                    .as_deref()
                    .or(Some(r.description.as_str()))
                    .filter(|s| !s.is_empty());
                if let Some(desc) = desc
                    && lines.len() < max_lines
                {
                    let indent_width = if *grouped { 6 } else { 2 };
                    let available = max_desc_width.saturating_sub(indent_width);
                    let truncated = truncate_str(desc, available);
                    let desc_indent = " ".repeat(indent_width);
                    lines.push(Line::from(Span::styled(
                        format!("{desc_indent}{truncated}"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        rows_rendered += 1;
        row_idx += 1;
    }

    app.visible_height = rows_rendered;

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Truncate a string to fit within `max_chars` columns, appending an ellipsis
/// if truncated. Uses char boundaries to avoid panics on multi-byte characters.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut char_count = 0;
    for (idx, _) in s.char_indices() {
        char_count += 1;
        if char_count > max_chars {
            // We need to truncate; find the byte position for (max_chars - 1) chars
            // and append ellipsis
            let truncate_at = s
                .char_indices()
                .nth(max_chars.saturating_sub(1))
                .map_or(idx, |(i, _)| i);
            return format!("{}\u{2026}", &s[..truncate_at]);
        }
    }
    // String fits entirely
    let _ = char_count;
    s.to_string()
}

fn render_detail(frame: &mut Frame, app: &mut SearchApp, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(selected) = app.rows.get(app.selected) else {
        let msg = Paragraph::new("No result selected.").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    };

    match selected {
        ListRow::Skill { result_idx, .. } => {
            let idx = *result_idx;
            render_skill_detail(frame, app, inner, idx);
        }
        ListRow::RepoHeader { .. } => render_repo_detail(frame, app, inner),
    }
}

fn render_skill_detail(frame: &mut Frame, app: &mut SearchApp, area: Rect, idx: usize) {
    let Some(r) = app.results.get(idx) else {
        return;
    };

    let owner = r
        .source
        .split_once('/')
        .map_or(r.source.as_str(), |(owner, _)| owner);
    let wrap_width = area.width.saturating_sub(2) as usize;

    let source_label = registry_label(&r.registry);
    let url = source_url(&r.registry, &r.source);
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Source: ", LABEL_STYLE),
            Span::styled(source_label, LINK_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Owner:  ", LABEL_STYLE),
            Span::styled(owner, VALUE_STYLE),
        ]),
    ];

    push_metric_lines(&mut lines, r.stars, r.weekly_installs);
    lines.push(Line::from(""));

    // Show skill description first (from SKILL.md), then repo description
    if let Some(ref skill_desc) = r.skill_description {
        push_wrapped_section(&mut lines, "Description:", skill_desc, wrap_width);
    } else if !r.description.is_empty() {
        push_wrapped_section(&mut lines, "Description:", &r.description, wrap_width);
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

    // Record hyperlink for post-draw OSC 8 emission.
    // "Source: " is 8 columns, so the label starts at area.x + 8.
    app.hyperlinks.push(Hyperlink {
        x: area.x + 8,
        y: area.y,
        text: source_label.to_string(),
        url,
    });
}

fn render_repo_detail(frame: &mut Frame, app: &mut SearchApp, area: Rect) {
    let ListRow::RepoHeader {
        owner_repo,
        stars,
        weekly_installs,
        description,
        skill_count,
        registry,
    } = &app.rows[app.selected]
    else {
        return;
    };

    let owner = owner_repo
        .split_once('/')
        .map_or(owner_repo.as_str(), |(o, _)| o)
        .to_string();
    let wrap_width = area.width.saturating_sub(2) as usize;

    let source_label = registry_label(registry).to_string();
    let url = source_url(registry, owner_repo);
    let owner_repo = owner_repo.clone();
    let stars = *stars;
    let weekly_installs = *weekly_installs;
    let description = description.clone();
    let skill_count = *skill_count;

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Source:  ", LABEL_STYLE),
            Span::styled(source_label.clone(), LINK_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Repo:    ", LABEL_STYLE),
            Span::styled(owner_repo.clone(), VALUE_STYLE),
        ]),
        Line::from(vec![
            Span::styled("Owner:   ", LABEL_STYLE),
            Span::styled(owner, VALUE_STYLE),
        ]),
    ];

    push_metric_lines(&mut lines, stars, weekly_installs);

    lines.push(Line::from(vec![
        Span::styled("Skills:  ", LABEL_STYLE),
        Span::styled(skill_count.to_string(), VALUE_STYLE),
    ]));

    push_wrapped_section(&mut lines, "Description:", &description, wrap_width);

    lines.push(Line::from(Span::styled("Install all:", LABEL_STYLE)));
    lines.push(Line::from(Span::styled(
        format!("  ion add {owner_repo}"),
        DIM_STYLE,
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);

    // Record hyperlink for post-draw OSC 8 emission.
    // "Source:  " is 9 columns.
    app.hyperlinks.push(Hyperlink {
        x: area.x + 9,
        y: area.y,
        text: source_label,
        url,
    });
}

/// Append a labeled wrapped-text section to `lines` if text is non-empty.
fn push_wrapped_section<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &'a str,
    text: &str,
    wrap_width: usize,
) {
    push_styled_section(lines, label, text, wrap_width, VALUE_STYLE);
}

/// Append a labeled wrapped section with a custom style.
fn push_styled_section<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &'a str,
    text: &str,
    wrap_width: usize,
    style: Style,
) {
    if text.is_empty() {
        return;
    }
    lines.push(Line::from(Span::styled(label, LABEL_STYLE)));
    for wrapped_line in wrap_text(text, wrap_width) {
        lines.push(Line::from(Span::styled(format!("  {wrapped_line}"), style)));
    }
    lines.push(Line::from(""));
}

/// Format a compact metric badge for the list view.
fn format_metric_compact(r: &ion_skill::search::SearchResult) -> String {
    if let Some(w) = r.weekly_installs {
        format!(" {w}/wk")
    } else if let Some(s) = r.stars {
        format!(" *{s}")
    } else {
        String::new()
    }
}

/// Format a compact metric badge for repo headers.
fn format_metric_compact_raw(stars: Option<u64>, weekly_installs: Option<u64>) -> String {
    if let Some(w) = weekly_installs {
        format!(" {w}/wk")
    } else if let Some(s) = stars {
        format!(" *{s}")
    } else {
        String::new()
    }
}

/// Push metric lines (stars and/or weekly installs) into the detail panel.
fn push_metric_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    stars: Option<u64>,
    weekly_installs: Option<u64>,
) {
    if let Some(s) = stars {
        lines.push(Line::from(vec![
            Span::styled("Stars:   ", LABEL_STYLE),
            Span::styled(s.to_string(), VALUE_STYLE),
        ]));
    }
    if let Some(w) = weekly_installs {
        lines.push(Line::from(vec![
            Span::styled("Installs:", LABEL_STYLE),
            Span::styled(format!(" {w}/wk"), VALUE_STYLE),
        ]));
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
