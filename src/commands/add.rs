use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::manifest_writer;
use ion_skill::source::{SkillSource, SourceType};

use crate::commands::install_shared::SkillEntry;
use crate::commands::validation::{confirm_proceed_with_collection, select_warned_skills};
use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(
    source_str: &str,
    rev: Option<&str>,
    bin: bool,
    json: bool,
    allow_warnings: bool,
    skills_filter: Option<&str>,
) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);

    let expanded = ctx.global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    if bin {
        source.source_type = SourceType::Binary;
        if source.binary.is_none() {
            source.binary = Some(source.display_name());
        }
    }

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    // Ensure the built-in ion-cli skill is deployed
    if let Err(e) = crate::builtin_skill::ensure_installed(
        &ctx.project_dir,
        &ctx.manifest_path,
        &merged_options,
    ) {
        log::warn!("Failed to install built-in ion-cli skill: {e}");
    }

    // Binary skills always install as a single skill — no collection fallback
    if bin {
        let name = source.display_name();
        println!(
            "Adding binary skill {} from {}...",
            p.bold(&format!("'{name}'")),
            p.info(source_str)
        );
        let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
        let locked = installer.install(&name, &source)?;
        return finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json);
    }

    // If the source has no path (i.e. points to a whole repo), check if it's
    // a multi-skill collection. Try to install as a single skill first; if there
    // is no root SKILL.md, discover and install all skills in the repo.
    if source.path.is_none() {
        let name = source.display_name();
        println!(
            "Adding skill {} from {}...",
            p.bold(&format!("'{name}'")),
            p.info(source_str)
        );

        let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
        match installer.install(&name, &source) {
            Ok(locked) => {
                return finish_single_install(
                    &ctx,
                    &p,
                    &merged_options,
                    &name,
                    &source,
                    locked,
                    json,
                );
            }
            Err(SkillError::ValidationWarning { report, .. }) => {
                crate::commands::install_shared::handle_validation_warnings(
                    &name,
                    &report,
                    json,
                    allow_warnings,
                )?;
                let locked = installer.install_with_options(
                    &name,
                    &source,
                    InstallValidationOptions {
                        skip_validation: false,
                        allow_warnings: true,
                    },
                )?;
                return finish_single_install(
                    &ctx,
                    &p,
                    &merged_options,
                    &name,
                    &source,
                    locked,
                    json,
                );
            }
            Err(SkillError::InvalidSkill(msg)) if msg.contains("No SKILL.md found") => {
                // Not a single-skill repo — try as a multi-skill collection
                return install_collection(
                    &ctx,
                    &p,
                    &merged_options,
                    &source,
                    source_str,
                    json,
                    allow_warnings,
                    skills_filter,
                );
            }
            Err(err) => return Err(err.into()),
        }
    }

    // Source has a path — install a single skill directly
    let name = source.display_name();
    println!(
        "Adding skill {} from {}...",
        p.bold(&format!("'{name}'")),
        p.info(source_str)
    );

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    let locked = crate::commands::install_shared::install_with_warning_prompt(
        &installer,
        &name,
        &source,
        json,
        allow_warnings,
    )?;

    finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json)
}

#[allow(clippy::too_many_arguments)]
fn install_collection(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    base_source: &SkillSource,
    source_str: &str,
    json: bool,
    allow_warnings: bool,
    skills_filter: Option<&str>,
) -> anyhow::Result<()> {
    use ion_skill::validate::ValidationReport;

    let skills = SkillInstaller::discover_skills(base_source)?;
    if skills.is_empty() {
        anyhow::bail!("No skills found in repository '{source_str}'");
    }

    println!(
        "Found {} skill(s) in collection {}:",
        p.bold(&skills.len().to_string()),
        p.info(source_str)
    );
    for (name, path) in &skills {
        println!("  {} {}", p.bold(name), p.dim(&format!("({path})")));
    }
    println!();

    if !json && !confirm_proceed_with_collection(skills.len())? {
        println!("Installation cancelled.");
        return Ok(());
    }

    // Phase 1: Validate all skills upfront
    let installer = SkillInstaller::new(&ctx.project_dir, merged_options);

    let mut clean: Vec<SkillEntry> = Vec::new();
    let mut warned: Vec<(SkillEntry, ValidationReport)> = Vec::new();
    let mut errored: Vec<(String, ValidationReport)> = Vec::new();

    println!("Validating skills...");
    for (name, path) in &skills {
        let mut skill_source = base_source.clone();
        skill_source.path = Some(path.clone());

        match installer.validate(name, &skill_source) {
            Ok(report) if report.warning_count > 0 => {
                warned.push((
                    SkillEntry {
                        name: name.clone(),
                        source: skill_source,
                    },
                    report,
                ));
            }
            Ok(_) => {
                clean.push(SkillEntry {
                    name: name.clone(),
                    source: skill_source,
                });
            }
            Err(SkillError::ValidationFailed { report, .. }) => {
                errored.push((name.clone(), report));
            }
            Err(e) => return Err(e.into()),
        }
    }

    // JSON mode: return skill list if no explicit selection provided
    if json && skills_filter.is_none() {
        let skills_data: Vec<_> = clean
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name, "status": "clean"
                })
            })
            .chain(warned.iter().map(|(e, r)| {
                serde_json::json!({
                    "name": e.name, "status": "warnings", "warning_count": r.warning_count
                })
            }))
            .chain(errored.iter().map(|(name, r)| {
                serde_json::json!({
                    "name": name, "status": "error", "error_count": r.error_count
                })
            }))
            .collect();

        crate::json::print_action_required(
            "skill_selection",
            serde_json::json!({
                "skills": skills_data
            }),
        );
    }

    let selected_names: Option<Vec<&str>> =
        skills_filter.map(|f| f.split(',').map(|s| s.trim()).collect());

    // Phase 2: Display validation summary
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
    for (name, report) in &errored {
        println!(
            "  {} {} — {} error(s), will be skipped",
            "✗",
            p.bold(name),
            report.error_count
        );
        for finding in &report.findings {
            println!(
                "      {} [{}] {}",
                finding.severity, finding.checker, finding.message
            );
        }
    }
    println!();

    // Phase 2b: Interactive selection for warned skills
    let warned_selections = if !warned.is_empty() {
        if json {
            if !allow_warnings {
                // In JSON mode without allow_warnings, check if any warned skills are selected
                if let Some(ref names) = selected_names {
                    let warned_in_selection: Vec<_> = warned
                        .iter()
                        .filter(|(e, _)| names.contains(&e.name.as_str()))
                        .collect();
                    if !warned_in_selection.is_empty() {
                        crate::json::print_action_required(
                            "validation_warnings",
                            serde_json::json!({
                                "skills": warned_in_selection.iter().map(|(e, r)| serde_json::json!({
                                    "name": e.name,
                                    "warnings": &r.findings,
                                })).collect::<Vec<_>>(),
                            }),
                        );
                    }
                }
            }
            // In JSON mode with allow_warnings: install all warned
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
    let mut lockfile = ctx.lockfile()?;
    let mut installed_count = 0;

    // Install clean skills (no prompt needed)
    for entry in &clean {
        if let Some(ref names) = selected_names
            && !names.contains(&entry.name.as_str())
        {
            continue;
        }
        println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions::default(),
        )?;

        finish_collection_skill_install(
            ctx,
            p,
            merged_options,
            &entry.name,
            &entry.source,
            &locked,
        )?;
        manifest_writer::add_skill(&ctx.manifest_path, &entry.name, &entry.source)?;
        lockfile.upsert(locked);
        installed_count += 1;
    }

    // Install user-approved warned skills
    for (i, (entry, _report)) in warned.iter().enumerate() {
        if !warned_selections[i] {
            println!("  Skipping '{}' (deselected)", entry.name);
            continue;
        }
        if let Some(ref names) = selected_names
            && !names.contains(&entry.name.as_str())
        {
            continue;
        }

        println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions {
                skip_validation: false,
                allow_warnings: true,
            },
        )?;

        finish_collection_skill_install(
            ctx,
            p,
            merged_options,
            &entry.name,
            &entry.source,
            &locked,
        )?;
        manifest_writer::add_skill(&ctx.manifest_path, &entry.name, &entry.source)?;
        lockfile.upsert(locked);
        installed_count += 1;
    }

    // Log skipped errored skills
    for (name, _) in &errored {
        println!("  Skipping '{}' (validation errors)", name);
    }

    // Register in global registry (once for the base source)
    crate::commands::install_shared::register_in_registry(base_source, &ctx.project_dir)?;

    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        // Collect names of installed skills
        let mut installed_names: Vec<String> = clean
            .iter()
            .filter(|e| {
                selected_names
                    .as_ref()
                    .is_none_or(|names| names.contains(&e.name.as_str()))
            })
            .map(|e| e.name.clone())
            .collect();
        for (i, (entry, _)) in warned.iter().enumerate() {
            if warned_selections[i]
                && selected_names
                    .as_ref()
                    .is_none_or(|names| names.contains(&entry.name.as_str()))
            {
                installed_names.push(entry.name.clone());
            }
        }
        crate::json::print_success(serde_json::json!({
            "installed": installed_names,
            "skipped": errored.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        }));
        return Ok(());
    }

    if installed_count > 0 {
        println!("  Updated {}", p.dim("Ion.toml"));
        println!("  Updated {}", p.dim("Ion.lock"));
    }
    println!(
        "{}",
        p.success(&format!(
            "Done! Installed {installed_count} of {} skill(s).",
            skills.len()
        ))
    );
    prompt_github_star(base_source);
    crate::commands::init::print_no_targets_hint(merged_options, p, json);
    Ok(())
}

/// Shared helper for per-skill post-install steps within a collection.
fn finish_collection_skill_install(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    _locked: &ion_skill::lockfile::LockedSkill,
) -> anyhow::Result<()> {
    println!(
        "    Installed to {}",
        p.info(&format!(".agents/skills/{name}/"))
    );
    for target_name in merged_options.targets.keys() {
        println!("    Linked to {}", p.info(target_name));
    }

    crate::commands::install_shared::add_gitignore_entries(
        &ctx.project_dir,
        name,
        source,
        merged_options,
    )?;

    Ok(())
}

fn finish_single_install(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
    json: bool,
) -> anyhow::Result<()> {
    // Add per-skill gitignore entries for remote skills only
    crate::commands::install_shared::add_gitignore_entries(
        &ctx.project_dir,
        name,
        source,
        merged_options,
    )?;

    // Register in global registry for git-based sources
    crate::commands::install_shared::register_in_registry(source, &ctx.project_dir)?;

    manifest_writer::add_skill(&ctx.manifest_path, name, source)?;

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "installed_to": format!(".agents/skills/{name}/"),
            "targets": merged_options.targets.keys().collect::<Vec<_>>(),
        }));
        return Ok(());
    }

    println!(
        "  Installed to {}",
        p.info(&format!(".agents/skills/{name}/"))
    );
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {}", p.info(target_name));
    }

    if source.source_type != SourceType::Path {
        println!("  Updated {}", p.dim(".gitignore"));
    }

    println!("  Updated {}", p.dim("Ion.toml"));
    println!("  Updated {}", p.dim("Ion.lock"));

    println!("{}", p.success("Done!"));
    prompt_github_star(source);
    crate::commands::init::print_no_targets_hint(merged_options, p, json);
    Ok(())
}

fn starred_repos_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("ion").join("starred.json"))
}

fn load_starred_repos() -> Vec<String> {
    starred_repos_path()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_starred_repos(repos: &[String]) {
    if let Some(path) = starred_repos_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(repos).unwrap_or_default(),
        );
    }
}

fn prompt_github_star(source: &SkillSource) {
    if source.source_type != SourceType::Github || !std::io::stdin().is_terminal() {
        return;
    }

    let repo = &source.source;
    let mut starred = load_starred_repos();
    if starred.iter().any(|r| r == repo) {
        return;
    }

    print!("  Star {repo} on GitHub? [Y/n] ");
    let _ = std::io::stdout().flush();

    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return;
    }
    let answer = answer.trim();
    if answer.is_empty() || answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
        let _ = std::process::Command::new("gh")
            .args(["repo", "star", repo])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // Record regardless of yes/no so we don't ask again
    starred.push(repo.to_string());
    save_starred_repos(&starred);
}

