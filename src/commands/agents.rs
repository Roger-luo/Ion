use std::io::{self, Write};

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

/// Result of applying an AGENTS.md template to a project.
pub(crate) struct TemplateSetup {
    /// Canonical template source recorded in Ion.toml (e.g. `builtin:rust`).
    pub template: String,
    /// True if a fresh AGENTS.md was written; false if AGENTS.md already
    /// existed and the template was staged as `.agents/templates/AGENTS.md.upstream`.
    pub created: bool,
}

/// Fetch a template and apply it to the project: write (or stage) AGENTS.md,
/// record `[agents]` config + lock entry, gitignore the upstream staging file,
/// create agent symlinks, and deploy the agents-update skill.
///
/// Performs no printing so it can be reused by both `agents init` (which prints
/// a human summary or a JSON envelope) and `ion init` (which folds the result
/// into its own output). The caller must reject a project that already has an
/// `[agents]` template configured.
pub(crate) fn apply_template(
    project: &Project,
    merged_options: &ion_skill::manifest::ManifestOptions,
    global_config: &GlobalConfig,
    source: &str,
    rev: Option<&str>,
    path: Option<&str>,
) -> anyhow::Result<TemplateSetup> {
    // Resolve built-in vs remote source and fetch the template content.
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
        (fetched, format!("builtin:{name}"))
    } else {
        let resolved_source = global_config.resolve_source(source);
        let fetched = ion_skill::agents::fetch_template(&resolved_source, rev, path, &project.dir)?;
        (fetched, source.to_string())
    };

    let agents_md_path = project.dir.join("AGENTS.md");
    let upstream_dir = project.dir.join(".agents/templates");
    let upstream_path = upstream_dir.join("AGENTS.md.upstream");
    let already_existed = agents_md_path.exists();

    if already_existed {
        // Existing AGENTS.md — stage upstream for merging rather than clobber it.
        std::fs::create_dir_all(&upstream_dir)?;
        std::fs::write(&upstream_path, &fetched.content)?;
    } else {
        std::fs::write(&agents_md_path, &fetched.content)?;
    }

    // Record [agents] config + lock entry.
    ion_skill::manifest_writer::write_agents_config(
        &project.manifest_path,
        &canonical_source,
        rev,
        path,
    )?;
    let mut lockfile = project.lockfile()?;
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: canonical_source.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&project.lockfile_path)?;

    // Gitignore the upstream staging file.
    let entries = [".agents/templates/AGENTS.md.upstream"];
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&project.dir, &entries)?;
    if !missing.is_empty() {
        let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        ion_skill::gitignore::append_to_gitignore(&project.dir, &refs)?;
    }

    // Create agent symlinks (e.g. CLAUDE.md -> AGENTS.md).
    if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&project.dir, &merged_options.targets)
    {
        log::warn!("Failed to create agent symlinks: {e}");
    }

    // Deploy the agents-update built-in skill.
    if let Err(e) = deploy_agents_update_skill(project, merged_options) {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }

    Ok(TemplateSetup {
        template: canonical_source,
        created: !already_existed,
    })
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

    let agents_md_existed = project.dir.join("AGENTS.md").exists();
    let merged_options = ws.merged_options_for(project)?;
    let setup = apply_template(
        project,
        &merged_options,
        &ws.global_config,
        source,
        rev,
        path,
    )?;

    if json {
        crate::json::print_success(serde_json::json!({
            "template": setup.template,
            "agents_md_created": setup.created,
        }));
        return Ok(());
    }

    if agents_md_existed {
        println!(
            "{} AGENTS.md already exists — upstream template staged to {}",
            p.warn("note:"),
            p.dim(".agents/templates/AGENTS.md.upstream")
        );
        println!("  Merge changes manually or ask your agent to help.");
    } else {
        println!("{} AGENTS.md from template", p.success("Created"));
    }
    println!("  {} Ion.toml with template source", p.success("Updated"));

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

/// Result of an AGENTS.md ↔ CLAUDE.md migration step.
#[derive(Debug)]
pub(crate) enum AgentsMdAction {
    /// Pointer file replaced or fresh symlink created.
    Symlinked,
    /// CLAUDE.md renamed to AGENTS.md, optional backup created.
    Renamed { backup: Option<String> },
    /// Both files have real content — user must resolve manually.
    Skipped { reason: String },
}

impl AgentsMdAction {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        match self {
            AgentsMdAction::Symlinked => serde_json::json!({"action": "symlinked"}),
            AgentsMdAction::Renamed { backup } => serde_json::json!({
                "action": "renamed",
                "from": "CLAUDE.md",
                "backup": backup,
            }),
            AgentsMdAction::Skipped { reason } => serde_json::json!({
                "action": "skipped",
                "reason": reason,
            }),
        }
    }
}

/// Convert a regular CLAUDE.md file into a symlink pointing at AGENTS.md.
///
/// Returns `Ok(None)` when there is nothing to do (no regular CLAUDE.md file
/// to migrate). The caller is responsible for gating on whether `claude` is
/// a configured target if that matters for the surrounding command.
pub(crate) fn migrate_claude_md(
    project_dir: &std::path::Path,
    p: &Paint,
    json: bool,
    yes: bool,
    rename_without_prompt: bool,
) -> anyhow::Result<Option<AgentsMdAction>> {
    let agents_path = project_dir.join("AGENTS.md");
    let claude_path = project_dir.join("CLAUDE.md");

    let agents_exists = agents_path.exists();

    let claude_meta = std::fs::symlink_metadata(&claude_path).ok();
    let claude_is_symlink = claude_meta.as_ref().is_some_and(|m| m.is_symlink());
    let claude_is_file = claude_meta.as_ref().is_some_and(|m| m.is_file());

    if claude_is_symlink || !claude_is_file {
        // No regular CLAUDE.md file to migrate — caller can run
        // `ensure_agent_symlinks` if it wants to create one from scratch.
        return Ok(None);
    }

    let claude_content = std::fs::read_to_string(&claude_path)?;
    let is_pointer = ion_skill::agents::is_agents_pointer(&claude_content);

    match (agents_exists, is_pointer) {
        // AGENTS.md exists + CLAUDE.md is a pointer → replace with symlink.
        (true, true) => {
            if !json {
                println!(
                    "  {} is a pointer to AGENTS.md — replacing with symlink",
                    p.dim("CLAUDE.md")
                );
            }
            std::fs::remove_file(&claude_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
            ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
            Ok(Some(AgentsMdAction::Symlinked))
        }

        // AGENTS.md exists + CLAUDE.md has real content → conflict.
        (true, false) => {
            if yes || json {
                let reason =
                    "Both AGENTS.md and CLAUDE.md have content — run without --yes to choose which to keep".to_string();
                if !json {
                    println!("  {}", p.dim(&reason));
                }
                return Ok(Some(AgentsMdAction::Skipped { reason }));
            }

            println!();
            println!("Both AGENTS.md and CLAUDE.md exist with content.");
            println!("  (1) Keep AGENTS.md (backup CLAUDE.md to CLAUDE.md.bak)");
            println!("  (2) Keep CLAUDE.md as AGENTS.md (backup AGENTS.md to AGENTS.md.bak)");
            println!("  (3) Skip — I'll handle this manually");
            print!("> ");
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let choice = answer.trim();

            match choice {
                "1" => {
                    std::fs::rename(&claude_path, project_dir.join("CLAUDE.md.bak"))?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                    ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                    if !json {
                        println!(
                            "  Kept AGENTS.md, backed up CLAUDE.md to {}",
                            p.dim("CLAUDE.md.bak")
                        );
                    }
                    Ok(Some(AgentsMdAction::Renamed {
                        backup: Some("CLAUDE.md.bak".to_string()),
                    }))
                }
                "2" => {
                    std::fs::rename(&agents_path, project_dir.join("AGENTS.md.bak"))?;
                    std::fs::rename(&claude_path, &agents_path)?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                    ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                    if !json {
                        println!(
                            "  Renamed CLAUDE.md to AGENTS.md, backed up old AGENTS.md to {}",
                            p.dim("AGENTS.md.bak")
                        );
                    }
                    Ok(Some(AgentsMdAction::Renamed {
                        backup: Some("AGENTS.md.bak".to_string()),
                    }))
                }
                _ => {
                    if !json {
                        println!("  Skipping AGENTS.md/CLAUDE.md migration.");
                    }
                    Ok(Some(AgentsMdAction::Skipped {
                        reason: "user chose to skip".to_string(),
                    }))
                }
            }
        }

        // No AGENTS.md + CLAUDE.md is a pointer → broken pointer, warn.
        (false, true) => {
            if !json {
                eprintln!("Warning: CLAUDE.md references @AGENTS.md but AGENTS.md does not exist.");
            }
            Ok(Some(AgentsMdAction::Skipped {
                reason: "pointer to nonexistent AGENTS.md".to_string(),
            }))
        }

        // No AGENTS.md + CLAUDE.md has real content → rename.
        (false, false) => {
            if yes || json {
                // Non-interactive. When the caller opts in (e.g. `ion init`),
                // perform the unambiguous rename directly; otherwise (e.g.
                // `ion migrate --yes`) leave the rename as an explicit choice.
                if rename_without_prompt {
                    std::fs::rename(&claude_path, &agents_path)?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                    ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                    if !json {
                        println!("  Renamed CLAUDE.md to AGENTS.md, created symlink.");
                    }
                    return Ok(Some(AgentsMdAction::Renamed { backup: None }));
                }
                let reason =
                    "CLAUDE.md has content but no AGENTS.md — run without --yes to confirm rename"
                        .to_string();
                if !json {
                    println!("  {}", p.dim(&reason));
                }
                return Ok(Some(AgentsMdAction::Skipped { reason }));
            }

            println!();
            println!("Found CLAUDE.md but no AGENTS.md.");
            print!("  Rename CLAUDE.md to AGENTS.md and create symlink? [Y/n] ");
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let answer = answer.trim();

            if answer.is_empty()
                || answer.eq_ignore_ascii_case("y")
                || answer.eq_ignore_ascii_case("yes")
            {
                std::fs::rename(&claude_path, &agents_path)?;
                #[cfg(unix)]
                std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                if !json {
                    println!("  Renamed CLAUDE.md to AGENTS.md, created symlink.");
                }
                Ok(Some(AgentsMdAction::Renamed { backup: None }))
            } else {
                if !json {
                    println!("  Skipping AGENTS.md/CLAUDE.md migration.");
                }
                Ok(Some(AgentsMdAction::Skipped {
                    reason: "user chose to skip".to_string(),
                }))
            }
        }
    }
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
