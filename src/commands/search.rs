use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    cascade_search, parallel_search, AgentSource, GitHubSource, RegistrySource, SearchResult,
    SearchSource,
};

pub fn run(
    query: &str,
    all: bool,
    agent: bool,
    interactive: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
    let config = GlobalConfig::load()?;
    let results = execute_search(&config, query, all, agent, source_filter, limit)?;

    if results.is_empty() {
        println!("No results found for '{query}'.");
        return Ok(());
    }

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
        return search_single_source(config, name, query, limit);
    }

    let mut sources = build_sources(config);
    if agent
        && let Some(s) = build_agent_source(config)
    {
        sources.push(Box::new(s));
    }

    if all {
        Ok(parallel_search(sources, query, limit))
    } else {
        Ok(cascade_search(sources, query, limit))
    }
}

fn build_sources(config: &GlobalConfig) -> Vec<Box<dyn SearchSource + Send>> {
    let mut sources: Vec<Box<dyn SearchSource + Send>> = Vec::new();

    for (name, reg) in &config.registries {
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
        return Ok(GitHubSource.search(query, limit)?);
    }
    if name == "agent" {
        if let Some(s) = build_agent_source(config) {
            return Ok(s.search(query, limit)?);
        }
        anyhow::bail!("No agent-command configured. Set it with: ion config set search.agent-command '<command>'");
    }
    if let Some(reg) = config.registries.get(name) {
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
    let mut current_registry = String::new();
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
        }
        if r.source.is_empty() {
            println!("  {}", r.description);
        } else {
            println!(
                "  {:<24} {:<44} {}",
                r.name, r.description, r.source
            );
        }
    }
}

fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    let installable: Vec<&SearchResult> = results.iter().filter(|r| !r.source.is_empty()).collect();
    if installable.is_empty() {
        println!("No installable results to select from.");
        return Ok(());
    }

    let items: Vec<String> = installable
        .iter()
        .map(|r| format!("{} — {}", r.name, r.description))
        .collect();

    let selection = dialoguer::Select::new()
        .with_prompt("Select a skill to install")
        .items(&items)
        .default(0)
        .interact_opt()?;

    if let Some(idx) = selection {
        let chosen = installable[idx];
        println!("\nInstalling '{}'...", chosen.name);
        let status = std::process::Command::new("ion")
            .arg("add")
            .arg(&chosen.source)
            .status()?;
        if !status.success() {
            anyhow::bail!("ion add failed");
        }
    }

    Ok(())
}
