use std::io::{self, IsTerminal, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use ion_skill::validate::ValidationReport;

pub fn print_validation_report(skill_name: &str, report: &ValidationReport) {
    println!("  Validation findings for '{skill_name}':");
    for finding in &report.findings {
        println!(
            "    {} [{}] {}",
            finding.severity, finding.checker, finding.message
        );
        if let Some(detail) = &finding.detail {
            println!("      {detail}");
        }
    }
    println!(
        "  Found: {} error(s), {} warning(s), {} info",
        report.error_count, report.warning_count, report.info_count
    );
}

pub fn confirm_install_on_warnings() -> anyhow::Result<bool> {
    print!("Install anyway? [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();

    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// Interactive multi-select for warned skills in a collection.
/// Each entry is `(skill_name, warning_count)`. All are selected by default.
/// Returns a `Vec<bool>` indicating which skills the user approved.
///
/// Falls back to sequential y/N prompts if stdin is not a terminal.
pub fn select_warned_skills(skills: &[(String, usize)]) -> anyhow::Result<Vec<bool>> {
    if skills.is_empty() {
        return Ok(vec![]);
    }

    // Non-interactive fallback
    if !io::stdin().is_terminal() {
        return fallback_select(skills);
    }

    interactive_select(skills)
}

/// Fallback: sequential y/N for each skill when there's no TTY.
fn fallback_select(skills: &[(String, usize)]) -> anyhow::Result<Vec<bool>> {
    let mut selected = Vec::with_capacity(skills.len());
    for (name, count) in skills {
        print!(
            "  Install '{}' with {} warning(s)? [Y/n] ",
            name, count
        );
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        let answer = answer.trim();
        // Default is yes (opt-out model)
        selected.push(answer.is_empty() || answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"));
    }
    Ok(selected)
}

/// Interactive crossterm-based multi-select.
fn interactive_select(skills: &[(String, usize)]) -> anyhow::Result<Vec<bool>> {
    let mut selected = vec![true; skills.len()];
    let mut cursor_pos: usize = 0;

    let mut stdout = io::stdout();

    // Enable raw mode for key-by-key input
    terminal::enable_raw_mode()?;

    // Draw initial state
    let result = run_select_loop(&mut stdout, skills, &mut selected, &mut cursor_pos);

    // Always restore terminal
    terminal::disable_raw_mode()?;

    // Move cursor below the widget
    write!(stdout, "\r\n")?;
    stdout.flush()?;

    result?;
    Ok(selected)
}

fn render_select(
    stdout: &mut io::Stdout,
    skills: &[(String, usize)],
    selected: &[bool],
    cursor_pos: usize,
    line_count: usize,
) -> anyhow::Result<()> {
    // Move up to overwrite previous render (if any)
    if line_count > 0 {
        write!(stdout, "{}", cursor::MoveUp(line_count as u16))?;
    }

    write!(stdout, "\r")?;
    // Header
    write!(
        stdout,
        "Select which warned skills to install (↑↓ move, space toggle, enter confirm, a toggle all):\r\n"
    )?;

    for (i, (name, count)) in skills.iter().enumerate() {
        let marker = if cursor_pos == i { ">" } else { " " };
        let check = if selected[i] { "x" } else { " " };
        let warning_label = if *count == 1 { "warning" } else { "warnings" };
        write!(
            stdout,
            "  {marker} [{check}] {name} ({count} {warning_label})\r\n"
        )?;
    }

    stdout.flush()?;
    Ok(())
}

fn run_select_loop(
    stdout: &mut io::Stdout,
    skills: &[(String, usize)],
    selected: &mut Vec<bool>,
    cursor_pos: &mut usize,
) -> anyhow::Result<()> {
    let total_lines = skills.len() + 1; // header + items

    // Initial render
    render_select(stdout, skills, selected, *cursor_pos, 0)?;

    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            // Ctrl+C cancels — deselect everything
            if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                for s in selected.iter_mut() {
                    *s = false;
                }
                return Ok(());
            }

            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *cursor_pos + 1 < skills.len() {
                        *cursor_pos += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    selected[*cursor_pos] = !selected[*cursor_pos];
                }
                KeyCode::Char('a') => {
                    let all_selected = selected.iter().all(|&s| s);
                    for s in selected.iter_mut() {
                        *s = !all_selected;
                    }
                }
                KeyCode::Enter => {
                    return Ok(());
                }
                _ => {}
            }

            render_select(stdout, skills, selected, *cursor_pos, total_lines)?;
        }
    }
}
