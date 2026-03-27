use crate::context::ProjectContext;
use crate::style::Paint;

const AGENTS_UPDATE_SKILL_CONTENT: &str = r#"---
name: agents-update
description: Merge upstream AGENTS.md template changes into local AGENTS.md
---

# AGENTS.md Template Update

When the user asks to update or merge their AGENTS.md with upstream changes, follow this process:

## Prerequisites

Check that `.agents/templates/AGENTS.md.upstream` exists. If it doesn't, tell the user to run `ion agents update` first.

## Process

1. Read `.agents/templates/AGENTS.md.upstream` (the new upstream template version)
2. Read `AGENTS.md` (the current local version)
3. Compare the two files and identify:
   - Sections added in upstream that don't exist locally
   - Sections modified in upstream that the user hasn't changed
   - Sections the user has customized (preserve these)
4. Intelligently merge:
   - Add new upstream sections
   - Update unchanged sections to match upstream
   - Preserve user customizations
   - Flag conflicts where both upstream and local changed the same section
5. Write the merged result to `AGENTS.md`
6. Inform the user what changed

## Guidelines

- Always preserve user customizations over upstream changes when they conflict
- Add clear comments if you're unsure about a merge decision
- Show the user what you changed before writing
"#;

pub fn deploy_agents_update_skill(
    ctx: &ProjectContext,
    options: &ion_skill::manifest::ManifestOptions,
) -> anyhow::Result<()> {
    use ion_skill::installer::{SkillInstaller, builtin_skills_dir};

    let skill_name = "agents-update";
    let global_dir = builtin_skills_dir().join(skill_name);
    let global_skill_md = global_dir.join("SKILL.md");

    // Write/update SKILL.md in global storage
    let needs_write = if global_skill_md.exists() {
        std::fs::read_to_string(&global_skill_md).ok().as_deref()
            != Some(AGENTS_UPDATE_SKILL_CONTENT)
    } else {
        true
    };

    if needs_write {
        std::fs::create_dir_all(&global_dir)?;
        std::fs::write(&global_skill_md, AGENTS_UPDATE_SKILL_CONTENT)?;
    }

    // Deploy symlinks
    let installer = SkillInstaller::new(&ctx.project_dir, options);
    installer.deploy(skill_name, &global_dir)?;

    // Gitignore the symlinks
    let target_paths: Vec<&str> = options.targets.values().map(|s| s.as_str()).collect();
    ion_skill::gitignore::add_skill_entries(
        &ctx.project_dir,
        skill_name,
        &target_paths,
        options.skills_dir_or_default(),
    )?;

    // Register as local skill in Ion.toml if not already present
    let content = std::fs::read_to_string(&ctx.manifest_path).unwrap_or_default();
    if !content.contains(&format!("{skill_name} ="))
        && !content.contains(&format!("\"{skill_name}\""))
    {
        let source = ion_skill::source::SkillSource::local();
        ion_skill::manifest_writer::add_skill(&ctx.manifest_path, skill_name, &source)?;
    }

    Ok(())
}

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

    // Deploy agents-update built-in skill
    if let Err(e) = deploy_agents_update_skill(&ctx, &merged_options) {
        eprintln!("Warning: failed to deploy agents-update skill: {e}");
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

pub fn update(json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let agents_config = manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No [agents] template configured in Ion.toml. Run `ion agents init <source>` first."
            )
        })?;

    let resolved_source = ctx.global_config.resolve_source(agents_config);
    let agents = manifest.agents.as_ref().unwrap();

    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        agents.rev.as_deref(),
        agents.path.as_deref(),
        &ctx.project_dir,
    )?;

    // Compare with locked checksum
    let lockfile = ctx.lockfile()?;
    let old_checksum = lockfile.agents.as_ref().map(|a| a.checksum.clone());
    let old_rev = lockfile
        .agents
        .as_ref()
        .and_then(|a| a.rev.as_deref())
        .unwrap_or("unknown")
        .to_string();

    if old_checksum.as_deref() == Some(fetched.checksum.as_str()) {
        if !json {
            println!("agents: {} up to date with upstream", p.dim("AGENTS.md"));
        }
        return Ok(());
    }

    // Stage the new upstream
    let upstream_dir = ctx.project_dir.join(".agents/templates");
    std::fs::create_dir_all(&upstream_dir)?;
    let upstream_path = upstream_dir.join("AGENTS.md.upstream");
    std::fs::write(&upstream_path, &fetched.content)?;

    let new_rev = fetched.rev.as_deref().unwrap_or("unknown").to_string();

    // Update lockfile
    let mut lockfile = lockfile;
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: agents_config.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": true,
            "old_rev": old_rev,
            "new_rev": new_rev,
            "upstream_path": upstream_path.display().to_string(),
        }));
    } else {
        println!(
            "agents: upstream template updated ({} → {})",
            p.dim(&old_rev[..7.min(old_rev.len())]),
            p.info(&new_rev[..7.min(new_rev.len())])
        );
        println!(
            "  upstream saved to {}",
            p.dim(".agents/templates/AGENTS.md.upstream")
        );
        println!("  run your agent to merge, or manually diff:");
        println!("    {}", p.bold("ion agents diff"));
    }

    Ok(())
}

/// Template update logic for use within `ion update`. Non-fatal — returns
/// errors for the caller to display as warnings.
pub fn update_template_non_fatal(
    ctx: &ProjectContext,
    lockfile: &mut ion_skill::lockfile::Lockfile,
    p: &Paint,
    json: bool,
) -> anyhow::Result<()> {
    let manifest = ctx.manifest()?;
    let agents_config = manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .ok_or_else(|| anyhow::anyhow!("no agents template configured"))?;

    let resolved_source = ctx.global_config.resolve_source(agents_config);
    let agents = manifest.agents.as_ref().unwrap();

    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        agents.rev.as_deref(),
        agents.path.as_deref(),
        &ctx.project_dir,
    )?;

    let old_checksum = lockfile.agents.as_ref().map(|a| a.checksum.clone());
    if old_checksum.as_deref() == Some(fetched.checksum.as_str()) {
        return Ok(()); // Unchanged — silent
    }

    let upstream_dir = ctx.project_dir.join(".agents/templates");
    std::fs::create_dir_all(&upstream_dir)?;
    std::fs::write(upstream_dir.join("AGENTS.md.upstream"), &fetched.content)?;

    let old_rev = lockfile
        .agents
        .as_ref()
        .and_then(|a| a.rev.as_deref())
        .unwrap_or("unknown")
        .to_string();
    let new_rev = fetched.rev.as_deref().unwrap_or("unknown").to_string();

    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: agents_config.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });

    if !json {
        println!(
            "  {} agents template: {} → {}",
            p.success("✓"),
            old_rev.get(..7).unwrap_or(&old_rev),
            p.info(new_rev.get(..7).unwrap_or(&new_rev))
        );
        println!(
            "    upstream saved to {}",
            p.dim(".agents/templates/AGENTS.md.upstream")
        );
    }

    Ok(())
}

pub fn diff() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let agents_md = ctx.project_dir.join("AGENTS.md");
    let upstream_path = ctx.project_dir.join(".agents/templates/AGENTS.md.upstream");

    if !upstream_path.exists() {
        anyhow::bail!("No upstream template staged. Run `ion agents update` first.");
    }

    if !agents_md.exists() {
        anyhow::bail!("No AGENTS.md found in project root.");
    }

    let local_content = std::fs::read_to_string(&agents_md)?;
    let upstream_content = std::fs::read_to_string(&upstream_path)?;

    if local_content == upstream_content {
        println!("AGENTS.md is up to date with upstream.");
        return Ok(());
    }

    use similar::TextDiff;

    let diff = TextDiff::from_lines(&local_content, &upstream_content);
    print!(
        "{}",
        diff.unified_diff()
            .header("local/AGENTS.md", "upstream/AGENTS.md")
    );
    Ok(())
}
