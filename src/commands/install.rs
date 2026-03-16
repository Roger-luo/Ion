use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller, hash_simple};
use ion_skill::lockfile::LockedSkill;
use ion_skill::manifest::Manifest;
use ion_skill::registry::Registry;
use ion_skill::source::SourceType;
use ion_skill::validate::ValidationReport;

use crate::commands::validation::{print_validation_report, select_warned_skills};
use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(json: bool, allow_warnings: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let merged_options = ctx.merged_options(&manifest);

    // Ensure the built-in ion-cli skill is deployed
    if let Err(e) = crate::builtin_skill::ensure_installed(
        &ctx.project_dir,
        &ctx.manifest_path,
        &merged_options,
    ) {
        log::warn!("Failed to install built-in ion-cli skill: {e}");
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

    let merged_options = ctx.merged_options(&manifest);

    if !json {
        println!(
            "Installing {} skill(s)...",
            p.bold(&manifest.skills.len().to_string())
        );
    }

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);

    // Phase 1: Validate all skills upfront
    struct SkillEntry {
        name: String,
        source: ion_skill::source::SkillSource,
    }

    let mut clean: Vec<SkillEntry> = Vec::new();
    let mut warned: Vec<(SkillEntry, ValidationReport)> = Vec::new();
    let mut errored: Vec<(String, ValidationReport)> = Vec::new();

    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;

        // Local skills bypass validation — deploy directly from project tree
        if source.source_type == SourceType::Local {
            let skills_dir = merged_options.skills_dir.as_deref().unwrap_or(".agents/skills");
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
            println!(
                "  {} {} — validation errors, will be skipped",
                "✗",
                p.bold(name)
            );
        }
        println!();
    }

    // Phase 2b: Interactive selection for warned skills
    let warned_selections = if !warned.is_empty() {
        if json && !allow_warnings {
            let skills_data: Vec<serde_json::Value> = warned
                .iter()
                .map(|(entry, report)| {
                    let findings: Vec<serde_json::Value> = report
                        .findings
                        .iter()
                        .map(|f| {
                            serde_json::json!({
                                "severity": f.severity.to_string(),
                                "checker": f.checker,
                                "message": f.message,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "name": entry.name,
                        "warning_count": report.warning_count,
                        "findings": findings,
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

        if !matches!(
            entry.source.source_type,
            SourceType::Path | SourceType::Local
        ) {
            let target_paths: Vec<&str> = merged_options
                .targets
                .values()
                .map(|s| s.as_str())
                .collect();
            ion_skill::gitignore::add_skill_entries(&ctx.project_dir, &entry.name, &target_paths)?;
        }

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

        if !matches!(
            entry.source.source_type,
            SourceType::Path | SourceType::Local
        ) {
            let target_paths: Vec<&str> = merged_options
                .targets
                .values()
                .map(|s| s.as_str())
                .collect();
            ion_skill::gitignore::add_skill_entries(&ctx.project_dir, &entry.name, &target_paths)?;
        }

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

fn register_in_registry(
    source: &ion_skill::source::SkillSource,
    project_dir: &std::path::Path,
) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git)
        && let Ok(url) = source.git_url()
    {
        let repo_hash = format!("{:x}", hash_simple(&url));
        let project_str = project_dir.display().to_string();
        let mut registry = Registry::load()?;
        registry.register(&repo_hash, &url, &project_str);
        registry.save()?;
    }
    Ok(())
}
