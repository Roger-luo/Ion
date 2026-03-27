use crate::context::ProjectContext;
use crate::style::Paint;

pub fn init(source: &str, rev: Option<&str>, path: Option<&str>, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);

    // Check if [agents] already exists
    let manifest = ctx.manifest_or_empty()?;
    if manifest.agents.is_some() {
        anyhow::bail!(
            "Template already configured in Ion.toml. \
             Use `ion agents update` to fetch the latest, \
             or edit [agents] in Ion.toml manually to change the source."
        );
    }

    // Resolve source through global config aliases
    let resolved_source = ctx.global_config.resolve_source(source);

    // Fetch template
    let fetched = ion_skill::agents::fetch_template(&resolved_source, rev, path, &ctx.project_dir)?;

    let agents_md_path = ctx.project_dir.join("AGENTS.md");
    let upstream_dir = ctx.project_dir.join(".agents/templates");
    let upstream_path = upstream_dir.join("AGENTS.md.upstream");
    let already_existed = agents_md_path.exists();

    if already_existed {
        // Existing AGENTS.md — stage upstream for merging
        std::fs::create_dir_all(&upstream_dir)?;
        std::fs::write(&upstream_path, &fetched.content)?;
        if !json {
            println!(
                "{} AGENTS.md already exists — upstream template staged to {}",
                p.warn("note:"),
                p.dim(".agents/templates/AGENTS.md.upstream")
            );
            println!("  Merge changes manually or ask your agent to help.");
        }
    } else {
        // No existing AGENTS.md — copy as starting point
        std::fs::write(&agents_md_path, &fetched.content)?;
        if !json {
            println!("{} AGENTS.md from template", p.success("Created"));
        }
    }

    // Write [agents] to Ion.toml
    ion_skill::manifest_writer::write_agents_config(&ctx.manifest_path, source, rev, path)?;

    // Write lock entry
    let mut lockfile = ctx.lockfile()?;
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: source.to_string(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&ctx.lockfile_path)?;

    // Add specific gitignore entry for the upstream staging file
    let entries = [".agents/templates/AGENTS.md.upstream"];
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&ctx.project_dir, &entries)?;
    if !missing.is_empty() {
        let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        ion_skill::gitignore::append_to_gitignore(&ctx.project_dir, &refs)?;
    }

    // Create agent symlinks (e.g. CLAUDE.md -> AGENTS.md)
    let merged_options = ctx.merged_options(&manifest);
    if let Err(e) =
        ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets)
    {
        eprintln!("Warning: failed to create agent symlinks: {e}");
    }

    if !json {
        println!("  {} Ion.toml with template source", p.success("Updated"));
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "template": source,
            "agents_md_created": !already_existed,
        }));
    }

    Ok(())
}

pub fn update(_json: bool) -> anyhow::Result<()> {
    // Placeholder — implemented in Task 9
    anyhow::bail!("not yet implemented")
}

pub fn diff() -> anyhow::Result<()> {
    // Placeholder — implemented in Task 10
    anyhow::bail!("not yet implemented")
}
