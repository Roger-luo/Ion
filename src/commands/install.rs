use ion_skill::lockfile::LockedSkill;

use crate::commands::install_shared::{
    FinalizeOptions, ValidationBuckets, finalize_skill_install, install_approved_skills,
};
use crate::commands::validation::{print_validation_summary, select_warned_skills};
use crate::context::ProjectContext;

pub fn run(json: bool, allow_warnings: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = ctx.paint();
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let merged_options = ctx.merged_options(&manifest);

    ctx.ensure_builtin_skill(&merged_options);

    // Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
    if let Err(e) =
        ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets)
    {
        log::warn!("Failed to create agent symlinks: {e}");
    }

    // Deploy agents-update skill if [agents] template is configured
    if manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .is_some()
        && let Err(e) = crate::commands::agents::deploy_agents_update_skill(&ctx, &merged_options)
    {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }

    let mut lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!({
                "installed": [],
                "skipped": [],
            }));
            return Ok(());
        }
        println!("No skills declared in Ion.toml.");
        return Ok(());
    }

    if !json {
        println!(
            "Installing {} skill(s)...",
            p.bold(&manifest.skills.len().to_string())
        );
    }

    let installer = ctx.installer(&merged_options);

    // Handle local skills first (they bypass validation)
    let mut non_local_skills = Vec::new();
    for (name, entry) in &manifest.skills {
        let source = entry.resolve()?;

        if source.is_local() {
            let skills_dir = merged_options.skills_dir_or_default();
            let local_skill_dir = ctx.project_dir.join(skills_dir).join(name);

            if !local_skill_dir.exists() {
                println!(
                    "  {} {} — local skill directory not found, skipping",
                    p.warn("⚠"),
                    p.bold(name),
                );
                continue;
            }

            println!("  Installing {}...", p.bold(&format!("'{name}'")));
            installer.deploy(name, &local_skill_dir)?;

            let mut locked_local =
                LockedSkill::local(name.clone()).with_source(source.source.clone());
            if let Ok(checksum) = ion_skill::git::checksum_dir(&local_skill_dir) {
                locked_local = locked_local.with_checksum(checksum);
            }
            lockfile.upsert(locked_local);

            continue;
        }

        non_local_skills.push((name.clone(), source));
    }

    // Phase 1: Validate all non-local skills upfront
    let buckets = ValidationBuckets::collect(&installer, non_local_skills)?;

    // Phase 2: Display validation summary
    if !buckets.is_empty() {
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
        } else if json && allow_warnings {
            // In JSON mode with allow_warnings, install all warned skills
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
                &ctx,
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
    let mut json_skipped: Vec<serde_json::Value> = Vec::new();
    for (name, _) in &buckets.errored {
        if !json {
            println!("  Skipping '{}' (validation errors)", name);
        }
        json_skipped.push(serde_json::json!({ "name": name, "reason": "validation_errors" }));
    }

    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        let mut json_installed: Vec<serde_json::Value> = buckets
            .clean
            .iter()
            .map(|e| serde_json::json!({ "name": e.name }))
            .collect();
        for (i, (entry, _)) in buckets.warned.iter().enumerate() {
            if warned_selections.get(i).copied().unwrap_or(false) {
                json_installed.push(serde_json::json!({ "name": entry.name }));
            } else {
                json_skipped
                    .push(serde_json::json!({ "name": entry.name, "reason": "deselected" }));
            }
        }
        crate::json::print_success(serde_json::json!({
            "installed": json_installed,
            "skipped": json_skipped,
        }));
        return Ok(());
    }

    println!("Updated {}", p.dim("Ion.lock"));
    println!("{}", p.success("Done!"));

    Ok(())
}
