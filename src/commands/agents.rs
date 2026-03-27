use crate::context::ProjectContext;
use ion_skill::agents_md::{self, WriteResult};
use ion_skill::manifest_writer;

/// Run `ion agents fetch [URL] [--force]`.
///
/// Fetches the org-standard AGENTS.md from `url` (falling back to the URL
/// configured in Ion.toml `[options] agents-md-url` or global config
/// `[agents] md-url`) and writes it to `AGENTS.md` in the project root.
///
/// If AGENTS.md already exists the managed section is replaced while
/// project-specific content is preserved.  Use `--force` to overwrite the
/// entire file when it was not created by ion.
pub fn run_fetch(url: Option<&str>, force: bool, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = crate::style::Paint::new(&ctx.global_config);

    // Resolve URL: argument → project config → global config
    let resolved_url = url
        .map(str::to_owned)
        .or_else(|| {
            ctx.manifest_or_empty()
                .ok()
                .and_then(|m| m.options.agents_md_url)
        })
        .or_else(|| ctx.global_config.agents.md_url.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No AGENTS.md URL specified.  Provide a URL argument or set \
                 'agents-md-url' in Ion.toml [options] or 'agents.md-url' in \
                 the global config (~/.config/ion/config.toml)."
            )
        })?;

    // Persist the URL to Ion.toml when it was provided as an argument
    // (so subsequent `ion agents update` runs know where to fetch from).
    if url.is_some() {
        manifest_writer::write_agents_md_url(&ctx.manifest_path, &resolved_url)?;
    }

    if !json {
        println!("Fetching AGENTS.md from {}", p.info(&resolved_url));
    }

    let fetched = agents_md::fetch_content(&resolved_url)?;
    let agents_md_path = ctx.project_dir.join("AGENTS.md");

    let result = if agents_md_path.exists() {
        // Attempt to update only the managed section; fall back to full
        // overwrite when --force is given.
        if force {
            agents_md::write_new(&fetched, &agents_md_path, true)?
        } else {
            agents_md::update_managed(&fetched, &agents_md_path)?
        }
    } else {
        agents_md::write_new(&fetched, &agents_md_path, false)?
    };

    if json {
        let status = match result {
            WriteResult::Created => "created",
            WriteResult::Updated => "updated",
            WriteResult::Unchanged => "unchanged",
        };
        crate::json::print_success(serde_json::json!({
            "status": status,
            "path": agents_md_path.display().to_string(),
            "url": resolved_url,
        }));
        return Ok(());
    }

    match result {
        WriteResult::Created => println!("{} AGENTS.md", p.success("Created")),
        WriteResult::Updated => println!("{} managed section in AGENTS.md", p.success("Updated")),
        WriteResult::Unchanged => println!(
            "{}: AGENTS.md managed section is already up-to-date",
            p.info("hint")
        ),
    }

    Ok(())
}

/// Run `ion agents update`.
///
/// Re-fetches the org-standard AGENTS.md from the configured URL and updates
/// the managed section in the local AGENTS.md, preserving project-specific
/// content outside the managed markers.
pub fn run_update(json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = crate::style::Paint::new(&ctx.global_config);

    let url = ctx
        .manifest_or_empty()
        .ok()
        .and_then(|m| m.options.agents_md_url)
        .or_else(|| ctx.global_config.agents.md_url.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No AGENTS.md URL configured.  Run `ion agents fetch <URL>` first to \
                 set the URL, or set 'agents-md-url' in Ion.toml [options]."
            )
        })?;

    if !json {
        println!("Fetching AGENTS.md from {}", p.info(&url));
    }

    let fetched = agents_md::fetch_content(&url)?;
    let agents_md_path = ctx.project_dir.join("AGENTS.md");
    let result = agents_md::update_managed(&fetched, &agents_md_path)?;

    if json {
        let status = match result {
            WriteResult::Created => "created",
            WriteResult::Updated => "updated",
            WriteResult::Unchanged => "unchanged",
        };
        crate::json::print_success(serde_json::json!({
            "status": status,
            "path": agents_md_path.display().to_string(),
            "url": url,
        }));
        return Ok(());
    }

    match result {
        WriteResult::Created => println!("{} AGENTS.md", p.success("Created")),
        WriteResult::Updated => println!("{} managed section in AGENTS.md", p.success("Updated")),
        WriteResult::Unchanged => println!(
            "{}: AGENTS.md managed section is already up-to-date",
            p.info("hint")
        ),
    }

    Ok(())
}
