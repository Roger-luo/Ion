use ion_skill::lockfile::LockedSkill;

use crate::commands::install_shared::{
    FinalizeOptions, ValidationBuckets, finalize_skill_install, install_approved_skills,
};
use crate::commands::validation::{print_validation_summary, select_warned_skills};
use crate::context::WorkspaceContext;

pub fn run(json: bool, allow_warnings: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let projects = ws.scoped_projects();
    let p = ws.paint();
    let multi = projects.len() > 1;

    let mut all_json_installed: Vec<serde_json::Value> = Vec::new();
    let mut all_json_skipped: Vec<serde_json::Value> = Vec::new();

    for project in &projects {
        if !project.has_manifest() {
            continue;
        }

        let manifest = project.manifest()?;
        let merged_options = ws.merged_options_for(project)?;

        ws.ensure_builtin_skill(project, &merged_options);

        // Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
        if let Err(e) =
            ion_skill::agents::ensure_agent_symlinks(&project.dir, &merged_options.targets)
        {
            log::warn!("Failed to create agent symlinks: {e}");
        }

        // Deploy agents-update skill if [agents] template is configured
        if manifest
            .agents
            .as_ref()
            .and_then(|a| a.template.as_ref())
            .is_some()
            && let Err(e) =
                crate::commands::agents::deploy_agents_update_skill(project, &merged_options)
        {
            log::warn!("Failed to deploy agents-update skill: {e}");
        }

        let mut lockfile = project.lockfile()?;

        if manifest.skills.is_empty() {
            continue;
        }

        if multi && !json {
            let label = project_label(project, &ws);
            println!("\n{}:", p.bold(&label));
        }

        if !json {
            println!(
                "Installing {} skill(s)...",
                p.bold(&manifest.skills.len().to_string())
            );
        }

        let installer = ws.installer_for(project, &merged_options);

        // Handle local skills first (they bypass validation)
        let mut non_local_skills = Vec::new();
        let mut json_local_installed: Vec<serde_json::Value> = Vec::new();
        let mut json_local_skipped: Vec<serde_json::Value> = Vec::new();
        for (name, entry) in &manifest.skills {
            let source = entry.resolve()?;

            if source.is_local() {
                // Use explicit path from Ion.toml if set, otherwise fall back to skills-dir
                let local_skill_dir = if let Some(ref path) = source.path {
                    project.dir.join(path)
                } else {
                    let skills_dir = merged_options.skills_dir_or_default();
                    project.dir.join(skills_dir).join(name)
                };

                if !local_skill_dir.exists() {
                    if !json {
                        println!(
                            "  {} {} — local skill directory not found, skipping",
                            p.warn("⚠"),
                            p.bold(name),
                        );
                    }
                    json_local_skipped.push(serde_json::json!({
                        "name": name,
                        "reason": "local_directory_not_found",
                    }));
                    continue;
                }

                if !json {
                    println!("  Installing {}...", p.bold(&format!("'{name}'")));
                }
                installer.deploy(name, &local_skill_dir)?;

                let mut locked_local =
                    LockedSkill::local(name.clone()).with_source(source.source.clone());
                if let Ok(checksum) = ion_skill::git::checksum_dir(&local_skill_dir) {
                    locked_local = locked_local.with_checksum(checksum);
                }
                lockfile.upsert(locked_local);
                json_local_installed.push(serde_json::json!({ "name": name }));

                continue;
            }

            non_local_skills.push((name.clone(), source));
        }

        // Phase 1: Validate all non-local skills upfront
        let buckets = ValidationBuckets::collect(&installer, non_local_skills)?;

        // Phase 2: Display validation summary
        if !json && !buckets.is_empty() {
            print_validation_summary(&p, &buckets);
        }

        // Phase 2b: Interactive selection for warned skills
        let warned_selections = if !buckets.warned.is_empty() {
            if json && !allow_warnings {
                let skills_data: Vec<serde_json::Value> = buckets
                    .warned
                    .iter()
                    .map(|(entry, report)| {
                        serde_json::json!({
                            "name": entry.name,
                            "warning_count": report.warning_count,
                            "findings": &report.findings,
                        })
                    })
                    .collect();
                crate::json::print_action_required(
                    "validation_warnings",
                    serde_json::json!({
                        "skills": skills_data,
                        "hint": "Re-run with --allow-warnings to install these skills",
                    }),
                );
            } else if allow_warnings {
                // In JSON mode, or human mode with --allow-warnings: install
                // all warned skills without prompting. The flag must skip
                // the interactive selection too, not just the JSON
                // action_required — otherwise it silently does nothing for
                // a human-driven (or non-TTY) `ion add`.
                vec![true; buckets.warned.len()]
            } else {
                let items: Vec<(String, usize)> = buckets
                    .warned
                    .iter()
                    .map(|(entry, report)| (entry.name.clone(), report.warning_count))
                    .collect();
                match select_warned_skills(&items)? {
                    Some(selections) => selections,
                    None => {
                        println!("Installation cancelled.");
                        return Ok(());
                    }
                }
            }
        } else {
            vec![]
        };

        // Phase 3: Install approved skills
        let _installed = install_approved_skills(
            &installer,
            &buckets,
            &warned_selections,
            &p,
            json,
            |name, source, locked| {
                finalize_skill_install(
                    project,
                    &merged_options,
                    name,
                    source,
                    locked,
                    &mut lockfile,
                    &FinalizeOptions::INSTALL,
                )
            },
        )?;

        // Log skipped errored skills
        let mut json_skipped: Vec<serde_json::Value> = json_local_skipped;
        for (name, _) in &buckets.errored {
            if !json {
                println!("  Skipping '{}' (validation errors)", name);
            }
            json_skipped.push(serde_json::json!({ "name": name, "reason": "validation_errors" }));
        }

        lockfile.write_to(&project.lockfile_path)?;

        if json {
            let mut json_installed: Vec<serde_json::Value> = json_local_installed;
            json_installed.extend(
                buckets
                    .clean
                    .iter()
                    .map(|e| serde_json::json!({ "name": e.name })),
            );
            for (i, (entry, _)) in buckets.warned.iter().enumerate() {
                if warned_selections.get(i).copied().unwrap_or(false) {
                    json_installed.push(serde_json::json!({ "name": entry.name }));
                } else {
                    json_skipped
                        .push(serde_json::json!({ "name": entry.name, "reason": "deselected" }));
                }
            }
            all_json_installed.extend(json_installed);
            all_json_skipped.extend(json_skipped);
        } else {
            println!("Updated {}", p.dim("Ion.lock"));
            println!("{}", p.success("Done!"));
        }
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "installed": all_json_installed,
            "skipped": all_json_skipped,
        }));
    } else if !projects.iter().any(|p| p.has_manifest()) {
        println!("No Ion.toml found. Run `ion init` to set up a project.");
    }

    Ok(())
}

/// Human-readable label for a project within a workspace.
fn project_label(project: &ion_skill::workspace::Project, ws: &WorkspaceContext) -> String {
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
