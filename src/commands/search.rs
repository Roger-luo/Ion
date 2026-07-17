use std::io::IsTerminal;

use crossterm::style::Stylize;
use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    AgentSource, GitHubSource, RegistrySource, SearchCache, SearchResult, SearchSource,
    SkillsShSource, enrich_results, owner_repo_of, parallel_search, skill_dir_name,
};

pub fn run(
    query: &str,
    agent: bool,
    json: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
    log::debug!(
        "search starting: query={query:?}, agent={agent}, json={json}, source={source_filter:?}, limit={limit}"
    );
    let config = GlobalConfig::load()?;
    log::debug!(
        "loaded config: {} registries, agent_command={:?}",
        config.registries.len(),
        config.search.agent_command
    );
    let mut results = execute_search(&config, query, agent, source_filter, limit)?;

    if results.is_empty() {
        log::debug!("no results found");
        if json {
            crate::json::print_success(serde_json::json!([]));
            return Ok(());
        }
        println!("No results found for '{query}'.");
        return Ok(());
    }

    log::debug!(
        "found {} total results, enriching GitHub results",
        results.len()
    );
    enrich_results(&mut results);

    if json {
        crate::json::print_success(&results);
        return Ok(());
    }

    // Human mode: TUI picker if TTY, otherwise plain text list
    if std::io::stdout().is_terminal() {
        pick_and_install(&results)?;
    } else {
        print_results(&results);
    }

    Ok(())
}

fn execute_search(
    config: &GlobalConfig,
    query: &str,
    agent: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    if let Some(name) = source_filter {
        log::debug!("searching single source: {name}");
        return search_single_source(config, name, query, limit);
    }

    let mut sources = build_sources(config);
    log::debug!(
        "built {} sources: {}",
        sources.len(),
        sources
            .iter()
            .map(|s| s.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if agent && let Some(s) = build_agent_source(config) {
        log::debug!(
            "adding agent source with command template: {:?}",
            s.command_template
        );
        sources.push(Box::new(s));
    }

    let cache = SearchCache::new();
    let max_age_secs = config
        .cache
        .max_age_days
        .map(|d| u64::from(d) * 86400)
        .unwrap_or(86400); // default: 1 day

    log::debug!(
        "running parallel search across {} sources (cache max_age={}s)",
        sources.len(),
        max_age_secs
    );
    Ok(parallel_search(
        sources,
        query,
        limit,
        cache.as_ref(),
        max_age_secs,
    ))
}

pub(crate) fn build_sources(config: &GlobalConfig) -> Vec<Box<dyn SearchSource + Send>> {
    let mut sources: Vec<Box<dyn SearchSource + Send>> = Vec::new();

    log::debug!("adding built-in skills.sh source");
    sources.push(Box::new(SkillsShSource));

    for (name, reg) in &config.registries {
        if reg.url.contains("skills.sh") {
            continue;
        }
        log::debug!(
            "adding registry source: {name} (url={}, default={:?})",
            reg.url,
            reg.default
        );
        if reg.default == Some(true) {
            sources.insert(
                1,
                Box::new(RegistrySource {
                    registry_name: name.clone(),
                    base_url: reg.url.clone(),
                }),
            );
        } else {
            sources.push(Box::new(RegistrySource {
                registry_name: name.clone(),
                base_url: reg.url.clone(),
            }));
        }
    }

    sources.push(Box::new(GitHubSource));
    sources
}

fn build_agent_source(config: &GlobalConfig) -> Option<AgentSource> {
    config.search.agent_command.as_ref().map(|cmd| AgentSource {
        command_template: cmd.clone(),
    })
}

fn search_single_source(
    config: &GlobalConfig,
    name: &str,
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    if name == "github" {
        log::debug!("searching GitHub for: {query:?}");
        return Ok(GitHubSource.search(query, limit)?);
    }
    if name == "agent" {
        if let Some(s) = build_agent_source(config) {
            log::debug!("searching agent with command: {:?}", s.command_template);
            return Ok(s.search(query, limit)?);
        }
        anyhow::bail!(
            "No agent-command configured. Set it with: ion config set search.agent-command '<command>'"
        );
    }
    if name == "skills.sh" || name == "skills-sh" {
        log::debug!("searching skills.sh");
        return Ok(SkillsShSource.search(query, limit)?);
    }
    if let Some(reg) = config.registries.get(name) {
        log::debug!("searching registry {name} at {}", reg.url);
        let source = RegistrySource {
            registry_name: name.to_string(),
            base_url: reg.url.clone(),
        };
        return Ok(source.search(query, limit)?);
    }
    anyhow::bail!(
        "Unknown source '{name}'. Available: {}",
        available_sources(config)
    );
}

fn available_sources(config: &GlobalConfig) -> String {
    let mut names: Vec<&str> = config.registries.keys().map(|s| s.as_str()).collect();
    if !names.iter().any(|n| n.contains("skills")) {
        names.insert(0, "skills.sh");
    }
    names.push("github");
    names.join(", ")
}

fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

fn print_results(results: &[SearchResult]) {
    let color = std::io::stdout().is_terminal();

    // Collect registries in order of first appearance
    let mut registries: Vec<&str> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in results {
        if seen.insert(r.registry.as_str()) {
            registries.push(&r.registry);
        }
    }

    let mut rank = 1usize;
    for (i, registry) in registries.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let registry_results: Vec<&SearchResult> =
            results.iter().filter(|r| &r.registry == registry).collect();
        let count = registry_results.len();
        println!(
            " {} ({} result{})",
            registry,
            count,
            if count == 1 { "" } else { "s" }
        );

        let groups = group_by_owner_repo_refs(&registry_results);

        for (gi, (owner_repo, group)) in groups.iter().enumerate() {
            if gi > 0 {
                println!();
            }

            if group.len() == 1 {
                print_single_result(group[0], rank, color);
            } else {
                print_repo_group(owner_repo, group, rank, color);
            }
            rank += 1;
        }
    }
}

/// Group references to results by owner/repo, preserving first-occurrence order.
fn group_by_owner_repo_refs<'a>(
    results: &[&'a SearchResult],
) -> Vec<(String, Vec<&'a SearchResult>)> {
    let mut groups: Vec<(String, Vec<&'a SearchResult>)> = Vec::new();
    let mut key_to_idx: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for r in results {
        let key = owner_repo_of(&r.source);
        if let Some(&idx) = key_to_idx.get(key) {
            groups[idx].1.push(r);
        } else {
            key_to_idx.insert(key, groups.len());
            groups.push((key.to_string(), vec![r]));
        }
    }
    groups
}

/// Build a web URL for the result's source.
fn source_url(registry: &str, source: &str) -> String {
    match registry {
        "skills.sh" | "skills-sh" => format!("https://skills.sh/{source}"),
        _ => format!("https://github.com/{source}"),
    }
}

/// Format the metrics line for a search result (stars and/or weekly installs).
fn format_metrics(r: &SearchResult) -> String {
    let mut parts = Vec::new();
    if let Some(s) = r.stars {
        parts.push(format!("{s} stars"));
    }
    if let Some(w) = r.weekly_installs {
        parts.push(format!("{w} installs/wk"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("  ({})", parts.join(", "))
    }
}

fn print_install_line(source: &str, registry: &str, color: bool) {
    let url = source_url(registry, source);
    let line = format!("ion add {source}");
    if color {
        println!("    {}  {}", line.grey(), url.dark_blue());
    } else {
        println!("    {line}  {url}");
    }
}

fn print_single_result(r: &SearchResult, rank: usize, color: bool) {
    if r.source.is_empty() {
        println!("  {}", r.description);
        return;
    }

    let metrics = format_metrics(r);
    if color {
        print!(
            "  {rank}. {}{}",
            r.name.clone().white().bold(),
            metrics.clone().grey()
        );
    } else {
        print!("  {rank}. {}{}", r.name, metrics);
    }
    println!();

    let desc = r.skill_description.as_deref().unwrap_or(&r.description);
    if !desc.is_empty() {
        let usable = terminal_width().saturating_sub(4).max(20);
        print_wrapped(desc, 4, usable, 2, color);
    }

    print_install_line(&r.source, &r.registry, color);
}

fn print_repo_group(owner_repo: &str, group: &[&SearchResult], rank: usize, color: bool) {
    let r0 = group[0];
    let skill_count = group.len();
    let width = terminal_width();

    let metrics = format_metrics(r0);
    let heading = format!(
        "{rank}. {}{}  ({} skill{})",
        owner_repo,
        metrics,
        skill_count,
        if skill_count == 1 { "" } else { "s" }
    );
    if color {
        println!("  {}", heading.white().bold());
    } else {
        println!("  {heading}");
    }

    // Show first available description from the group
    let desc = group
        .iter()
        .find_map(|r| r.skill_description.as_deref())
        .or_else(|| {
            let d = r0.description.as_str();
            if d.is_empty() { None } else { Some(d) }
        });
    if let Some(desc) = desc {
        let usable = width.saturating_sub(4).max(20);
        print_wrapped(desc, 4, usable, 2, color);
    }

    let skill_names: Vec<&str> = group.iter().map(|r| skill_dir_name(&r.source)).collect();
    let skills_line = truncate_list(&skill_names, width.saturating_sub(4));
    if color {
        println!("    {}", skills_line.cyan());
    } else {
        println!("    {skills_line}");
    }

    print_install_line(owner_repo, &r0.registry, color);
}

/// Join names with ", " and truncate with "..." if it exceeds `max_width`.
fn truncate_list(items: &[&str], max_width: usize) -> String {
    let full = items.join(", ");
    if full.len() <= max_width {
        return full;
    }
    let mut result = String::new();
    for (i, item) in items.iter().enumerate() {
        let sep = if i == 0 { "" } else { ", " };
        let suffix = ", ...";
        if result.len() + sep.len() + item.len() + suffix.len() > max_width {
            result.push_str(suffix);
            return result;
        }
        result.push_str(sep);
        result.push_str(item);
    }
    result
}

fn print_wrapped(text: &str, indent: usize, width: usize, max_lines: usize, color: bool) {
    let prefix: String = " ".repeat(indent);
    let lines = crate::tui::util::wrap_text(text, width);

    for (i, line) in lines.iter().take(max_lines).enumerate() {
        let is_last_allowed = i + 1 == max_lines;
        let has_more = i + 1 < lines.len();

        let display = if is_last_allowed && has_more {
            truncate_for_ellipsis(line, width)
        } else {
            line.clone()
        };

        if color {
            println!("{prefix}{}", display.cyan());
        } else {
            println!("{prefix}{display}");
        }
    }
}

/// Truncate `line` to fit within `width` (byte) columns and append "...".
/// Always cuts on a UTF-8 char boundary so multi-byte characters (e.g. CJK
/// text that shows up in skill descriptions from the registry) are never
/// split — slicing mid-character would panic ("byte index is not a char
/// boundary").
fn truncate_for_ellipsis(line: &str, width: usize) -> String {
    let limit = width.saturating_sub(3);
    let truncated = if line.len() > limit {
        &line[..floor_char_boundary(line, limit)]
    } else {
        line
    };
    format!("{truncated}...")
}

/// Find the largest byte index `<= index` that lies on a UTF-8 char boundary
/// of `s`. Used so we can safely slice `s` at `index` without panicking when
/// `index` falls inside a multi-byte character.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    let mut i = index.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    use std::io::Write;

    use crossterm::cursor::MoveTo;
    use crossterm::event::{self, Event};
    use crossterm::queue;
    use crossterm::style::{Print, SetAttribute, SetForegroundColor};

    use crate::tui::search_app::SearchApp;
    use crate::tui::search_event::handle_search_key;
    use crate::tui::search_ui::render_search;
    use crate::tui::terminal::run_tui;

    let installable: Vec<SearchResult> = results
        .iter()
        .filter(|r| !r.source.is_empty())
        .cloned()
        .collect();
    if installable.is_empty() {
        println!("No installable results to select from.");
        return Ok(());
    }

    let mut app = SearchApp::new(installable);

    run_tui(|terminal| {
        loop {
            terminal.draw(|frame| render_search(frame, &mut app))?;

            // Emit OSC 8 hyperlinks after draw — ratatui doesn't support
            // hyperlinks natively, so we re-print the link text wrapped in
            // OSC 8 sequences at the known positions.
            {
                let writer = terminal.backend_mut();
                for link in &app.hyperlinks {
                    queue!(
                        writer,
                        MoveTo(link.x, link.y),
                        Print(format!("\x1b]8;;{}\x07", link.url)),
                        SetForegroundColor(crossterm::style::Color::Blue),
                        SetAttribute(crossterm::style::Attribute::Underlined),
                        Print(&link.text),
                        SetAttribute(crossterm::style::Attribute::NoUnderline),
                        Print("\x1b]8;;\x07"),
                    )?;
                }
                writer.flush()?;
            }

            if let Event::Key(key) = event::read()? {
                handle_search_key(&mut app, key)?;
            }

            if app.should_quit || app.should_install {
                break;
            }
        }
        Ok(())
    })?;

    // Drain any leftover key events from the TUI (e.g. the Enter that triggered install)
    while crossterm::event::poll(std::time::Duration::from_millis(10))? {
        let _ = crossterm::event::read();
    }

    if app.should_install
        && let Some(source) = app.selected_install_source()
    {
        log::debug!("user selected install source: {source}");
        crate::commands::add::run(source, None, false, false, None, false, false, None, &[])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression test for a real panic hit while dogfooding `ion search`:
    // a skills.sh result had a CJK description whose wrap width landed the
    // truncation index in the middle of a multi-byte character, causing
    // "byte index N is not a char boundary" in `print_wrapped`.
    #[test]
    fn truncate_for_ellipsis_does_not_split_multibyte_char() {
        // "支持四引擎的PDF..." — each CJK char is 3 bytes in UTF-8. A byte-width
        // limit can easily fall inside one of these characters.
        let line = "支持四引擎的PDF处理工具支持四引擎的PDF处理工具";
        for width in 0..40 {
            // Must not panic for any width.
            let result = truncate_for_ellipsis(line, width);
            assert!(result.ends_with("..."));
        }
    }

    #[test]
    fn truncate_for_ellipsis_ascii_truncates_and_adds_ellipsis() {
        let result = truncate_for_ellipsis("hello world", 8);
        // limit = 8 - 3 = 5 bytes -> "hello" + "..."
        assert_eq!(result, "hello...");
    }

    #[test]
    fn truncate_for_ellipsis_short_line_still_gets_ellipsis() {
        // Mirrors print_wrapped's behavior: when this is the last allowed
        // line and there's more content beyond it, "..." is appended even
        // if the line itself already fits within the limit.
        let result = truncate_for_ellipsis("hi", 80);
        assert_eq!(result, "hi...");
    }

    #[test]
    fn floor_char_boundary_snaps_back_from_mid_character() {
        let s = "a支b"; // 'a' (1 byte), '支' (3 bytes), 'b' (1 byte)
        // Index 2 and 3 both fall inside/after '支' (which spans bytes 1..4).
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 1), 1); // right after 'a'
        assert_eq!(floor_char_boundary(s, 2), 1); // inside '支' -> snaps back
        assert_eq!(floor_char_boundary(s, 3), 1); // still inside '支'
        assert_eq!(floor_char_boundary(s, 4), 4); // right after '支'
        assert_eq!(floor_char_boundary(s, 100), s.len()); // clamps to len
    }

    #[test]
    fn print_wrapped_does_not_panic_on_cjk_description() {
        // End-to-end: this is the exact call shape used by print_single_result
        // / print_repo_group with a description that previously panicked.
        let desc = "支持四引擎的PDF处理工具，可以提取文本和表格并转换格式的强大助手";
        print_wrapped(desc, 4, 20, 2, false);
    }
}
