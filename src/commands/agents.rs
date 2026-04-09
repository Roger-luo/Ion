use crate::context::WorkspaceContext;
use crate::style::Paint;
use ion_skill::config::GlobalConfig;
use ion_skill::workspace::Project;

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
    project: &Project,
    options: &ion_skill::manifest::ManifestOptions,
) -> anyhow::Result<()> {
    use ion_skill::installer::builtin_skills_dir;

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
    let installer = ion_skill::installer::SkillInstaller::new(&project.dir, options);
    installer.deploy(skill_name, &global_dir)?;

    // Gitignore the symlinks
    let target_paths: Vec<&str> = options.targets.values().map(|s| s.as_str()).collect();
    ion_skill::gitignore::add_skill_entries(
        &project.dir,
        skill_name,
        &target_paths,
        options.skills_dir_or_default(),
    )?;

    // Register as local skill in Ion.toml if not already present
    let content = std::fs::read_to_string(&project.manifest_path).unwrap_or_default();
    if !content.contains(&format!("{skill_name} ="))
        && !content.contains(&format!("\"{skill_name}\""))
    {
        let source = ion_skill::source::SkillSource::local();
        ion_skill::manifest_writer::add_skill(&project.manifest_path, skill_name, &source)?;
    }

    Ok(())
}

pub fn init(
    source: &str,
    rev: Option<&str>,
    path: Option<&str>,
    json: bool,
    project_flags: &[String],
) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let project = ws.single_project()?;
    let p = ws.paint();

    // Check if [agents] already exists
    let manifest = project.manifest_or_empty()?;
    if manifest.agents.is_some() {
        anyhow::bail!(
            "Template already configured in Ion.toml. \
             Use `ion agents update` to fetch the latest, \
             or edit [agents] in Ion.toml manually to change the source."
        );
    }

    // Detect built-in templates and normalize the source name
    let (fetched, canonical_source) = if let Some(name) =
        ion_skill::templates::parse_builtin_name(source)
    {
        if rev.is_some() {
            anyhow::bail!("--rev cannot be used with built-in templates");
        }
        if path.is_some() {
            anyhow::bail!("--path cannot be used with built-in templates");
        }
        let fetched = ion_skill::agents::fetch_builtin_template(name)?;
        let canonical = format!("builtin:{name}");
        (fetched, canonical)
    } else {
        let resolved_source = ws.global_config.resolve_source(source);
        let fetched = ion_skill::agents::fetch_template(&resolved_source, rev, path, &project.dir)?;
        (fetched, source.to_string())
    };

    let agents_md_path = project.dir.join("AGENTS.md");
    let upstream_dir = project.dir.join(".agents/templates");
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
    ion_skill::manifest_writer::write_agents_config(
        &project.manifest_path,
        &canonical_source,
        rev,
        path,
    )?;

    // Write lock entry
    let mut lockfile = project.lockfile()?;
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: canonical_source.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&project.lockfile_path)?;

    // Add specific gitignore entry for the upstream staging file
    let entries = [".agents/templates/AGENTS.md.upstream"];
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&project.dir, &entries)?;
    if !missing.is_empty() {
        let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        ion_skill::gitignore::append_to_gitignore(&project.dir, &refs)?;
    }

    // Create agent symlinks (e.g. CLAUDE.md -> AGENTS.md)
    let merged_options = ws.merged_options_for(project)?;
    if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&project.dir, &merged_options.targets)
    {
        log::warn!("Failed to create agent symlinks: {e}");
    }

    // Deploy agents-update built-in skill
    if let Err(e) = deploy_agents_update_skill(project, &merged_options) {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }

    if !json {
        println!("  {} Ion.toml with template source", p.success("Updated"));
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "template": canonical_source,
            "agents_md_created": !already_existed,
        }));
    }

    Ok(())
}

pub fn update(json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let projects = ws.scoped_projects();
    let p = ws.paint();
    let multi = projects.len() > 1;

    let mut any_updated = false;
    let mut json_results: Vec<serde_json::Value> = Vec::new();

    for project in &projects {
        if !project.has_manifest() {
            continue;
        }

        let manifest = project.manifest()?;
        let agents_config = match manifest.agents.as_ref().and_then(|a| a.template.as_ref()) {
            Some(config) => config.clone(),
            None => {
                // In multi-project mode, silently skip projects without [agents]
                if !multi {
                    anyhow::bail!(
                        "No [agents] template configured in Ion.toml. Run `ion agents init <source>` first."
                    );
                }
                continue;
            }
        };

        if multi && !json {
            let label = project_label(project, &ws);
            println!("\n{}:", p.bold(&label));
        }

        let resolved_source = ws.global_config.resolve_source(&agents_config);
        let agents = manifest.agents.as_ref().unwrap();

        let fetched = ion_skill::agents::fetch_template(
            &resolved_source,
            agents.rev.as_deref(),
            agents.path.as_deref(),
            &project.dir,
        )?;

        // Compare with locked checksum
        let lockfile = project.lockfile()?;
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
            continue;
        }

        any_updated = true;

        // Stage the new upstream
        let upstream_dir = project.dir.join(".agents/templates");
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
        lockfile.write_to(&project.lockfile_path)?;

        if json {
            json_results.push(serde_json::json!({
                "project": project_label(project, &ws),
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
    }

    if json {
        if json_results.is_empty() && !any_updated {
            crate::json::print_success(serde_json::json!({
                "updated": false,
            }));
        } else {
            crate::json::print_success(serde_json::json!({
                "results": json_results,
            }));
        }
    }

    Ok(())
}

/// Template update logic for use within `ion update`. Non-fatal — returns
/// errors for the caller to display as warnings.
pub fn update_template_non_fatal(
    project: &Project,
    global_config: &GlobalConfig,
    lockfile: &mut ion_skill::lockfile::Lockfile,
    p: &Paint,
    json: bool,
) -> anyhow::Result<()> {
    let manifest = project.manifest()?;
    let agents_config = manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .ok_or_else(|| anyhow::anyhow!("no agents template configured"))?;

    let resolved_source = global_config.resolve_source(agents_config);
    let agents = manifest.agents.as_ref().unwrap();

    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        agents.rev.as_deref(),
        agents.path.as_deref(),
        &project.dir,
    )?;

    let old_checksum = lockfile.agents.as_ref().map(|a| a.checksum.clone());
    if old_checksum.as_deref() == Some(fetched.checksum.as_str()) {
        return Ok(()); // Unchanged — silent
    }

    let upstream_dir = project.dir.join(".agents/templates");
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

pub fn diff(project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let projects = ws.scoped_projects();
    let p = ws.paint();
    let multi = projects.len() > 1;

    let mut any_diff = false;

    for project in &projects {
        let agents_md = project.dir.join("AGENTS.md");
        let upstream_path = project.dir.join(".agents/templates/AGENTS.md.upstream");

        if !upstream_path.exists() {
            if !multi {
                anyhow::bail!("No upstream template staged. Run `ion agents update` first.");
            }
            continue;
        }

        if !agents_md.exists() {
            if !multi {
                anyhow::bail!("No AGENTS.md found in project root.");
            }
            continue;
        }

        if multi {
            let label = project_label(project, &ws);
            println!("\n{}:", p.bold(&label));
        }

        let local_content = std::fs::read_to_string(&agents_md)?;
        let upstream_content = std::fs::read_to_string(&upstream_path)?;

        if local_content == upstream_content {
            println!("AGENTS.md is up to date with upstream.");
            continue;
        }

        any_diff = true;
        use similar::TextDiff;

        let diff = TextDiff::from_lines(&local_content, &upstream_content);
        print!(
            "{}",
            diff.unified_diff()
                .header("local/AGENTS.md", "upstream/AGENTS.md")
        );
    }

    if multi && !any_diff {
        println!("All AGENTS.md files are up to date with upstream.");
    }

    Ok(())
}

/// Human-readable label for a project within a workspace.
fn project_label(project: &Project, ws: &WorkspaceContext) -> String {
    let root_dir = ws.root_dir();
    if project.dir == root_dir {
        ". (root)".to_string()
    } else {
        project
            .dir
            .strip_prefix(root_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| project.dir.display().to_string())
    }
}
