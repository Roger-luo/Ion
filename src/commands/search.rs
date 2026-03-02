use std::io::IsTerminal;

use crossterm::style::Stylize;
use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    enrich_github_results, owner_repo_of, parallel_search, AgentSource,
    GitHubSource, RegistrySource, SearchResult, SearchSource, SkillsShSource,
};

pub fn run(
    query: &str,
    all: bool,
    agent: bool,
    interactive: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
    log::debug!("search starting: query={query:?}, all={all}, agent={agent}, interactive={interactive}, source={source_filter:?}, limit={limit}");
    let config = GlobalConfig::load()?;
    log::debug!("loaded config: {} registries, agent_command={:?}", config.registries.len(), config.search.agent_command);
    let mut results = execute_search(&config, query, all, agent, source_filter, limit)?;

    if results.is_empty() {
        log::debug!("no results found");
        println!("No results found for '{query}'.");
        return Ok(());
    }

    log::debug!("found {} total results, enriching GitHub results", results.len());
    enrich_github_results(&mut results);
    print_results(&results);

    if interactive {
        pick_and_install(&results)?;
    }

    Ok(())
}

fn execute_search(
    config: &GlobalConfig,
    query: &str,
    all: bool,
    agent: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    if let Some(name) = source_filter {
        log::debug!("searching single source: {name}");
        return search_single_source(config, name, query, limit);
    }

    let mut sources = build_sources(config);
    log::debug!("built {} sources: {}", sources.len(), sources.iter().map(|s| s.name()).collect::<Vec<_>>().join(", "));
    if agent
        && let Some(s) = build_agent_source(config)
    {
        log::debug!("adding agent source with command template: {:?}", s.command_template);
        sources.push(Box::new(s));
    }

    // Always search all sources in parallel and merge results.
    // This ensures skills.sh results are combined with GitHub results
    // rather than one source hiding the other.
    let _ = all; // flag kept for CLI compat but all sources always searched
    log::debug!("running parallel search across {} sources", sources.len());
    Ok(parallel_search(sources, query, limit))
}

fn build_sources(config: &GlobalConfig) -> Vec<Box<dyn SearchSource + Send>> {
    let mut sources: Vec<Box<dyn SearchSource + Send>> = Vec::new();

    // Always include skills.sh as the primary source (first in cascade order)
    log::debug!("adding built-in skills.sh source");
    sources.push(Box::new(SkillsShSource));

    for (name, reg) in &config.registries {
        // Skip skills.sh entries in config — we already have the built-in source
        if reg.url.contains("skills.sh") {
            continue;
        }
        log::debug!("adding registry source: {name} (url={}, default={:?})", reg.url, reg.default);
        if reg.default == Some(true) {
            sources.insert(
                1, // after skills.sh but before others
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
        anyhow::bail!("No agent-command configured. Set it with: ion config set search.agent-command '<command>'");
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
    anyhow::bail!("Unknown source '{name}'. Available: {}", available_sources(config));
}

fn available_sources(config: &GlobalConfig) -> String {
    let mut names: Vec<&str> = config.registries.keys().map(|s| s.as_str()).collect();
    if !names.iter().any(|n| n.contains("skills")) {
        names.insert(0, "skills.sh");
    }
    names.push("github");
    names.join(", ")
}

/// A group of results sharing the same owner/repo.
struct ResultGroup<'a> {
    owner_repo: &'a str,
    results: Vec<&'a SearchResult>,
}

/// Group results by owner/repo, preserving order of first occurrence.
fn group_by_repo<'a>(results: &'a [SearchResult]) -> Vec<ResultGroup<'a>> {
    let mut groups: Vec<ResultGroup<'a>> = Vec::new();
    let mut index_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for r in results {
        let key = owner_repo_of(&r.source);
        if let Some(&idx) = index_map.get(key) {
            groups[idx].results.push(r);
        } else {
            index_map.insert(key, groups.len());
            groups.push(ResultGroup {
                owner_repo: key,
                results: vec![r],
            });
        }
    }
    groups
}

fn print_results(results: &[SearchResult]) {
    let color = std::io::stdout().is_terminal();
    let mut current_registry = String::new();

    // Collect registries in order of first appearance
    let mut registries: Vec<&str> = Vec::new();
    for r in results {
        if !registries.contains(&r.registry.as_str()) {
            registries.push(&r.registry);
        }
    }

    for registry in registries {
        if !current_registry.is_empty() {
            println!();
        }
        current_registry = registry.to_string();

        let registry_results: Vec<&SearchResult> =
            results.iter().filter(|r| r.registry == registry).collect();
        let count = registry_results.len();
        println!(
            " {} ({} result{})",
            registry,
            count,
            if count == 1 { "" } else { "s" }
        );

        let registry_slice: Vec<SearchResult> =
            registry_results.into_iter().cloned().collect();
        let groups = group_by_repo(&registry_slice);

        let mut first_in_group = true;
        for group in &groups {
            if !first_in_group {
                println!();
            }
            first_in_group = false;

            if group.results.len() == 1 {
                print_single_result(group.results[0], color);
            } else {
                print_repo_group(group, color);
            }
        }
    }
}

fn print_single_result(r: &SearchResult, color: bool) {
    if r.source.is_empty() {
        println!("  {}", r.description);
        return;
    }

    // Line 1: name + stars
    let stars = match r.stars {
        Some(n) => format!("  * {n}"),
        None => String::new(),
    };
    if color {
        print!("  {}{}", r.name.clone().white().bold(), stars.white().bold());
    } else {
        print!("  {}{}", r.name, stars);
    }
    println!();

    // Line 2-3: description
    let desc = r.skill_description.as_deref().unwrap_or(&r.description);
    if !desc.is_empty() {
        let indent = 4;
        let max_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        let usable = max_width.saturating_sub(indent).max(20);
        print_wrapped(desc, indent, usable, 2, color);
    }

    // Line 3: install command
    if color {
        println!("    {}", format!("ion add {}", r.source).grey());
    } else {
        println!("    ion add {}", r.source);
    }
}

fn print_repo_group(group: &ResultGroup, color: bool) {
    let r0 = group.results[0];
    let skill_count = group.results.len();

    // Line 1: owner/repo  * stars  (N skills)
    let stars = match r0.stars {
        Some(n) => format!("  * {n}"),
        None => String::new(),
    };
    let heading = format!(
        "{}{}  ({} skill{})",
        group.owner_repo,
        stars,
        skill_count,
        if skill_count == 1 { "" } else { "s" }
    );
    if color {
        println!("  {}", heading.white().bold());
    } else {
        println!("  {heading}");
    }

    // Line 2: repo description
    if !r0.description.is_empty() {
        let indent = 4;
        let max_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        let usable = max_width.saturating_sub(indent).max(20);
        print_wrapped(&r0.description, indent, usable, 2, color);
    }

    // Line 3: comma-separated skill names
    let skill_names: Vec<&str> = group
        .results
        .iter()
        .map(|r| {
            // Extract skill directory name from source: "owner/repo/path/skill" → "skill"
            let after_repo = r.source.strip_prefix(group.owner_repo).unwrap_or(&r.source);
            let after_repo = after_repo.strip_prefix('/').unwrap_or(after_repo);
            after_repo.rsplit('/').next().unwrap_or(after_repo)
        })
        .collect();
    let max_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    let skills_line = truncate_list(&skill_names, max_width.saturating_sub(4));
    if color {
        println!("    {}", skills_line.cyan());
    } else {
        println!("    {skills_line}");
    }

    // Line 4: install command (whole repo)
    if color {
        println!("    {}", format!("ion add {}", group.owner_repo).grey());
    } else {
        println!("    ion add {}", group.owner_repo);
    }
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
            use crossterm::style::Stylize;
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

    // Handle install (after terminal is restored)
    if app.should_install
        && let Some(source) = app.selected_install_source()
    {
        log::debug!("user selected install source: {source}");
        println!("\nInstalling '{source}'...");
        let status = std::process::Command::new("ion")
            .arg("add")
            .arg(source)
            .status()?;
        if !status.success() {
            anyhow::bail!("ion add failed");
        }
    }

    Ok(())
}
