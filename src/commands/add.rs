use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::source::SkillSource;

use crate::commands::install_shared::{
    FinalizeOptions, ValidationBuckets, finalize_skill_install, finalize_skill_install_and_write,
    install_approved_skills, register_in_registry,
};
use crate::commands::validation::{
    confirm_proceed_with_collection, print_validation_summary, select_warned_skills,
};
use crate::context::ProjectContext;
use crate::style::Paint;

#[allow(clippy::too_many_arguments)]
pub fn run(
    source_str: &str,
    rev: Option<&str>,
    bin: bool,
    dev: bool,
    name_override: Option<&str>,
    json: bool,
    allow_warnings: bool,
    skills_filter: Option<&str>,
) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = ctx.paint();

    let mut bin = bin;

    if dev && !bin {
        anyhow::bail!("--dev can only be used with --bin for local binary skills");
    }

    let expanded = ctx.global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    // Auto-detect binary project from the target's Ion.toml [project] section
    let is_local_path = source.is_local_path();
    if !bin && is_local_path {
        let project_ion_toml = PathBuf::from(&source.source).join("Ion.toml");
        if let Some(meta) = ion_skill::manifest::read_project_meta(&project_ion_toml)
            && meta.is_binary()
        {
            bin = true;
            let binary_name = meta.binary.clone().unwrap_or_else(|| source.display_name());
            source = source.with_binary(binary_name);
        }
    }

    if bin {
        if !source.is_binary() {
            // Convert to binary kind with a default binary name
            let binary_name = source.display_name();
            source = source.with_binary(binary_name);
        }

        if dev {
            if !is_local_path {
                anyhow::bail!(
                    "--dev can only be used with local path sources (e.g., ./my-project)"
                );
            }
            source = source.with_dev(true);
        }

        // For local binary skills, resolve the binary name from cargo metadata
        // if not explicitly overridden
        let has_binary_name = matches!(
            &source.kind,
            ion_skill::source::SkillSourceKind::Binary { binary_name, .. } if !binary_name.is_empty()
        );
        if is_local_path && has_binary_name {
            // Already has a binary name from auto-detection or flag
        } else if is_local_path {
            let project_path = std::path::PathBuf::from(&source.source);
            let info = ion_skill::binary::cargo_project_info(&project_path)?;
            source = source.with_binary(info.binary_name);
        }
    }

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    ctx.ensure_builtin_skill(&merged_options);

    // Binary skills always install as a single skill — no collection fallback
    if bin {
        let name = name_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| match &source.kind {
                ion_skill::source::SkillSourceKind::Binary { binary_name, .. }
                    if !binary_name.is_empty() =>
                {
                    binary_name.clone()
                }
                _ => source.display_name(),
            });
        let mode = if dev { " (dev)" } else { "" };
        println!(
            "Adding binary skill {}{} from {}...",
            p.bold(&format!("'{name}'")),
            mode,
            p.info(source_str)
        );
        let installer = ctx.installer(&merged_options);
        let locked = installer.install(&name, &source)?;
        return finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json);
    }

    // If the source has no path (i.e. points to a whole repo), check if it's
    // a multi-skill collection. Try to install as a single skill first; if there
    // is no root SKILL.md, discover and install all skills in the repo.
    if source.path.is_none() {
        let name = name_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| source.display_name());
        println!(
            "Adding skill {} from {}...",
            p.bold(&format!("'{name}'")),
            p.info(source_str)
        );

        let installer = ctx.installer(&merged_options);
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
                // Check if the cloned repo declares itself as a binary project
                if let Some(repo_dir) = ion_skill::installer::cached_repo_path(&source) {
                    let repo_ion_toml = repo_dir.join("Ion.toml");
                    if let Some(meta) = ion_skill::manifest::read_project_meta(&repo_ion_toml)
                        && meta.is_binary()
                    {
                        let binary_name =
                            meta.binary.clone().unwrap_or_else(|| source.display_name());
                        let bin_source = source.clone().with_binary(binary_name);
                        let bin_name =
                            name_override.map(|s| s.to_string()).unwrap_or_else(
                                || match &bin_source.kind {
                                    ion_skill::source::SkillSourceKind::Binary {
                                        binary_name,
                                        ..
                                    } if !binary_name.is_empty() => binary_name.clone(),
                                    _ => bin_source.display_name(),
                                },
                            );
                        println!(
                            "Detected binary skill project, installing {} from {}...",
                            p.bold(&format!("'{bin_name}'")),
                            p.info(source_str)
                        );
                        let installer = ctx.installer(&merged_options);
                        let locked = installer.install(&bin_name, &bin_source)?;
                        return finish_single_install(
                            &ctx,
                            &p,
                            &merged_options,
                            &bin_name,
                            &bin_source,
                            locked,
                            json,
                        );
                    }
                }

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
    let name = name_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| source.display_name());
    println!(
        "Adding skill {} from {}...",
        p.bold(&format!("'{name}'")),
        p.info(source_str)
    );

    let installer = ctx.installer(&merged_options);
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
    let installer = ctx.installer(merged_options);

    println!("Validating skills...");
    let buckets = ValidationBuckets::collect(
        &installer,
        skills.iter().map(|(name, path)| {
            let mut s = base_source.clone();
            s.path = Some(path.clone());
            (name.clone(), s)
        }),
    )?;

    // JSON mode: return skill list if no explicit selection provided
    if json && skills_filter.is_none() {
        let skills_data: Vec<_> = buckets
            .clean
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name, "status": "clean"
                })
            })
            .chain(buckets.warned.iter().map(|(e, r)| {
                serde_json::json!({
                    "name": e.name, "status": "warnings", "warning_count": r.warning_count
                })
            }))
            .chain(buckets.errored.iter().map(|(name, r)| {
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
    print_validation_summary(p, &buckets);

    // Phase 2b: Interactive selection for warned skills
    let warned_selections = if !buckets.warned.is_empty() {
        if json {
            if !allow_warnings {
                // In JSON mode without allow_warnings, check if any warned skills are selected
                if let Some(ref names) = selected_names {
                    let warned_in_selection: Vec<_> = buckets
                        .warned
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
    let mut lockfile = ctx.lockfile()?;

    let installed_count = install_approved_skills(
        &installer,
        &buckets,
        &warned_selections,
        p,
        json,
        |name, source, locked| {
            println!(
                "    Installed to {}",
                p.info(&format!(".agents/skills/{name}/"))
            );
            for target_name in merged_options.targets.keys() {
                println!("    Linked to {}", p.info(target_name));
            }
            finalize_skill_install(
                ctx,
                merged_options,
                name,
                source,
                locked,
                &mut lockfile,
                &FinalizeOptions::ADD_COLLECTION,
            )
        },
    )?;

    // Log skipped errored skills
    for (name, _) in &buckets.errored {
        println!("  Skipping '{}' (validation errors)", name);
    }

    // Register in global registry (once for the base source)
    register_in_registry(base_source, &ctx.project_dir)?;

    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        // Collect names of installed skills
        let mut installed_names: Vec<String> = buckets
            .clean
            .iter()
            .filter(|e| {
                selected_names
                    .as_ref()
                    .is_none_or(|names| names.contains(&e.name.as_str()))
            })
            .map(|e| e.name.clone())
            .collect();
        for (i, (entry, _)) in buckets.warned.iter().enumerate() {
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
            "skipped": buckets.errored.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
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

fn finish_single_install(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
    json: bool,
) -> anyhow::Result<()> {
    finalize_skill_install_and_write(
        ctx,
        merged_options,
        name,
        source,
        locked,
        &FinalizeOptions::ADD,
    )?;

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

    if !source.is_path() {
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
    if !source.is_github() || !std::io::stdin().is_terminal() {
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
        let _ = ionem::shell::gh::star_repo(repo);
    }

    // Record regardless of yes/no so we don't ask again
    starred.push(repo.to_string());
    save_starred_repos(&starred);
}
