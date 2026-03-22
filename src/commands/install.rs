use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::lockfile::LockedSkill;
use ion_skill::source::SourceType;
use ion_skill::validate::ValidationReport;

use crate::commands::install_shared::{SkillEntry, add_gitignore_entries, register_in_registry};
use crate::commands::validation::{print_validation_report, select_warned_skills};
use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(json: bool, allow_warnings: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let merged_options = ctx.merged_options(&manifest);

    ctx.ensure_builtin_skill(&merged_options);

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

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);

    // Phase 1: Validate all skills upfront
    let mut clean: Vec<SkillEntry> = Vec::new();
    let mut warned: Vec<(SkillEntry, ValidationReport)> = Vec::new();
    let mut errored: Vec<(String, ValidationReport)> = Vec::new();

    for (name, entry) in &manifest.skills {
        let source = entry.resolve()?;

        // Local skills bypass validation — deploy directly from project tree
        if source.source_type == SourceType::Local {
            let skills_dir = merged_options
                .skills_dir
                .as_deref()
                .unwrap_or(".agents/skills");
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

            let checksum = ion_skill::git::checksum_dir(&local_skill_dir).ok();
            lockfile.upsert(LockedSkill {
                name: name.clone(),
                source: source.source.clone(),
                path: None,
                version: None,
                commit: None,
                checksum,
                binary: None,
                binary_version: None,
                binary_checksum: None,
                dev: None,
            });

            continue;
        }

        match installer.validate(name, &source) {
            Ok(report) if report.warning_count > 0 => {
                warned.push((
                    SkillEntry {
                        name: name.clone(),
                        source,
                    },
                    report,
                ));
            }
            Ok(_) => {
                clean.push(SkillEntry {
                    name: name.clone(),
                    source,
                });
            }
            Err(SkillError::ValidationFailed { report, .. }) => {
                print_validation_report(name, &report);
                errored.push((name.clone(), report));
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Phase 2: Display validation summary
    if !clean.is_empty() || !warned.is_empty() || !errored.is_empty() {
        for entry in &clean {
            println!("  {} {} — passed", p.success("✓"), p.bold(&entry.name));
        }
        for (entry, report) in &warned {
            println!(
                "  {} {} — {} warning(s)",
                p.warn("⚠"),
                p.bold(&entry.name),
                report.warning_count
            );
            for finding in &report.findings {
                println!(
                    "      {} [{}] {}",
                    finding.severity, finding.checker, finding.message
                );
            }
        }
        for (name, _) in &errored {
            println!("  ✗ {} — validation errors, will be skipped", p.bold(name));
        }
        println!();
    }

    // Phase 2b: Interactive selection for warned skills
    let warned_selections = if !warned.is_empty() {
        if json && !allow_warnings {
            let skills_data: Vec<serde_json::Value> = warned
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
            vec![true; warned.len()]
        } else {
            let items: Vec<(String, usize)> = warned
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
    let mut json_installed: Vec<serde_json::Value> = Vec::new();
    let mut json_skipped: Vec<serde_json::Value> = Vec::new();

    // Install clean skills
    for entry in &clean {
        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions::default(),
        )?;

        add_gitignore_entries(
            &ctx.project_dir,
            &entry.name,
            &entry.source,
            &merged_options,
        )?;
        register_in_registry(&entry.source, &ctx.project_dir)?;
        lockfile.upsert(locked);
        json_installed.push(serde_json::json!({ "name": entry.name }));
    }

    // Install user-approved warned skills
    for (i, (entry, _report)) in warned.iter().enumerate() {
        if !warned_selections[i] {
            if !json {
                println!("  Skipping '{}' (deselected)", entry.name);
            }
            json_skipped.push(serde_json::json!({ "name": entry.name, "reason": "deselected" }));
            continue;
        }

        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions {
                skip_validation: false,
                allow_warnings: true,
            },
        )?;

        add_gitignore_entries(
            &ctx.project_dir,
            &entry.name,
            &entry.source,
            &merged_options,
        )?;
        register_in_registry(&entry.source, &ctx.project_dir)?;
        lockfile.upsert(locked);
        json_installed.push(serde_json::json!({ "name": entry.name }));
    }

    // Log skipped errored skills
    for (name, _) in &errored {
        if !json {
            println!("  Skipping '{}' (validation errors)", name);
        }
        json_skipped.push(serde_json::json!({ "name": name, "reason": "validation_errors" }));
    }

    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
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
