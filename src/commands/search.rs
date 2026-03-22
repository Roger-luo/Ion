use std::io::IsTerminal;

use crossterm::style::Stylize;
use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    AgentSource, GitHubSource, RegistrySource, SearchCache, SearchResult, SearchSource,
    SkillsShSource, enrich_github_results, owner_repo_of, parallel_search, skill_dir_name,
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
    enrich_github_results(&mut results);

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
                print_single_result(group[0], color);
            } else {
                print_repo_group(owner_repo, group, color);
            }
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

fn format_stars(stars: Option<u64>) -> String {
    match stars {
        Some(n) => format!("  * {n}"),
        None => String::new(),
    }
}

fn print_install_line(source: &str, color: bool) {
    let line = format!("ion add {source}");
    if color {
        println!("    {}", line.grey());
    } else {
        println!("    {line}");
    }
}

fn print_single_result(r: &SearchResult, color: bool) {
    if r.source.is_empty() {
        println!("  {}", r.description);
        return;
    }

    let stars = format_stars(r.stars);
    if color {
        print!(
            "  {}{}",
            r.name.clone().white().bold(),
            stars.white().bold()
        );
    } else {
        print!("  {}{}", r.name, stars);
    }
    println!();

    let desc = r.skill_description.as_deref().unwrap_or(&r.description);
    if !desc.is_empty() {
        let usable = terminal_width().saturating_sub(4).max(20);
        print_wrapped(desc, 4, usable, 2, color);
    }

    print_install_line(&r.source, color);
}

fn print_repo_group(owner_repo: &str, group: &[&SearchResult], color: bool) {
    let r0 = group[0];
    let skill_count = group.len();
    let width = terminal_width();

    let stars = format_stars(r0.stars);
    let heading = format!(
        "{}{}  ({} skill{})",
        owner_repo,
        stars,
        skill_count,
        if skill_count == 1 { "" } else { "s" }
    );
    if color {
        println!("  {}", heading.white().bold());
    } else {
        println!("  {heading}");
    }

    if !r0.description.is_empty() {
        let usable = width.saturating_sub(4).max(20);
        print_wrapped(&r0.description, 4, usable, 2, color);
    }

    let skill_names: Vec<&str> = group.iter().map(|r| skill_dir_name(&r.source)).collect();
    let skills_line = truncate_list(&skill_names, width.saturating_sub(4));
    if color {
        println!("    {}", skills_line.cyan());
    } else {
        println!("    {skills_line}");
    }

    print_install_line(owner_repo, color);
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
            let limit = width.saturating_sub(3);
            let truncated = if line.len() > limit {
                &line[..limit]
            } else {
                line.as_str()
            };
            format!("{truncated}...")
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

fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    use crossterm::event::{self, Event};

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
        crate::commands::add::run(source, None, false, false, None, false, false, None)?;
    }

    Ok(())
}
