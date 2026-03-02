use std::io::IsTerminal;

use crossterm::style::Stylize;
use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    cascade_search, enrich_github_results, parallel_search, AgentSource, GitHubSource,
    RegistrySource, SearchResult, SearchSource,
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

    if all {
        log::debug!("running parallel search across {} sources", sources.len());
        Ok(parallel_search(sources, query, limit))
    } else {
        log::debug!("running cascade search across {} sources", sources.len());
        Ok(cascade_search(sources, query, limit))
    }
}

fn build_sources(config: &GlobalConfig) -> Vec<Box<dyn SearchSource + Send>> {
    let mut sources: Vec<Box<dyn SearchSource + Send>> = Vec::new();

    for (name, reg) in &config.registries {
        log::debug!("adding registry source: {name} (url={}, default={:?})", reg.url, reg.default);
        if reg.default == Some(true) {
            sources.insert(
                0,
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
    names.push("github");
    names.join(", ")
}

fn print_results(results: &[SearchResult]) {
    let color = std::io::stdout().is_terminal();
    let mut current_registry = String::new();
    let mut first_in_group = true;

    for r in results {
        if r.registry != current_registry {
            if !current_registry.is_empty() {
                println!();
            }
            let count = results.iter().filter(|x| x.registry == r.registry).count();
            println!(
                " {} ({} result{})",
                r.registry,
                count,
                if count == 1 { "" } else { "s" }
            );
            current_registry = r.registry.clone();
            first_in_group = true;
        }

        if !first_in_group {
            println!();
        }
        first_in_group = false;

        if r.source.is_empty() {
            // Agent results: just show the description
            println!("  {}", r.description);
            continue;
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

        // Line 2-3: description (prefer skill_description over repo description)
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
    use std::io;

    use crossterm::event::{self, Event};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    use crate::tui::search_app::SearchApp;
    use crate::tui::search_event::handle_search_key;
    use crate::tui::search_ui::render_search;

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

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|frame| render_search(frame, &mut app))?;

        if let Event::Key(key) = event::read()? {
            handle_search_key(&mut app, key)?;
        }

        if app.should_quit || app.should_install {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle install
    if app.should_install {
        if let Some(chosen) = app.selected_result() {
            log::debug!("user selected: {} (source={})", chosen.name, chosen.source);
            println!("\nInstalling '{}'...", chosen.name);
            let status = std::process::Command::new("ion")
                .arg("add")
                .arg(&chosen.source)
                .status()?;
            if !status.success() {
                anyhow::bail!("ion add failed");
            }
        }
    }

    Ok(())
}
