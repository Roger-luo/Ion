use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::commands::install_shared::register_in_registry;
use crate::context::ProjectContext;
use crate::style::Paint;
use ion_skill::installer::InstallValidationOptions;
use ion_skill::manifest::ManifestOptions;
use ion_skill::migrate::{
    DiscoveredSkill, DiscoveryOrigin, MigrateOptions, ResolvedSkill, discover_from_directories,
    discover_from_lockfile, discover_leftover_skills, move_skill_to_local,
};
use ion_skill::search::{SearchCache, SearchSource};

pub fn run(from: Option<&str>, dry_run: bool, json: bool, yes: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = ctx.paint();
    let project_dir = &ctx.project_dir;
    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let lockfile_path = from
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.project_dir.join("skills-lock.json"));

    // ── Phase 1: Discover skills ──────────────────────────────────────────
    let discovered = if lockfile_path.exists() {
        let skills = discover_from_lockfile(&lockfile_path)?;
        if !json {
            println!("Found skills-lock.json with {} skills.", skills.len());
        }
        skills
    } else {
        if !json {
            println!("No skills-lock.json found, scanning directories...");
        }
        let skills = discover_from_directories(project_dir)?;
        if skills.is_empty() {
            if json {
                crate::json::print_success(serde_json::json!({
                    "migrated": [],
                    "matched": [],
                    "custom": [],
                    "skipped": [],
                }));
                return Ok(());
            }
            println!("No skills found in .agents/skills/ or .claude/skills/.");
            return Ok(());
        }
        if !json {
            println!("Found {} skills in skill directories.", skills.len());
        }
        skills
    };

    if discovered.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!({
                "migrated": [],
                "matched": [],
                "custom": [],
                "skipped": [],
            }));
            return Ok(());
        }
        println!("No skills to migrate.");
        return Ok(());
    }

    // ── Phase 2: Resolve skills ───────────────────────────────────────────
    let mut resolved = Vec::new();
    let mut skipped = Vec::new();

    if yes || json {
        // Auto-resolve: use known sources, skip unknown
        for skill in &discovered {
            if let Some(source) = &skill.source {
                resolved.push(ResolvedSkill {
                    name: skill.name.clone(),
                    source: source.clone(),
                    rev: None,
                });
            } else {
                skipped.push(skill.name.clone());
            }
        }
    } else {
        // Interactive resolution
        let stdin = io::stdin();
        let mut stdin_lock = stdin.lock();
        for skill in &discovered {
            match resolve_skill(skill, &mut stdin_lock) {
                Some(r) => resolved.push(r),
                None => skipped.push(skill.name.clone()),
            }
        }
    }

    // In JSON mode without --yes, report the plan and exit
    if json && !yes {
        let plan: Vec<_> = resolved
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "source": format_source(&s.source),
                    "rev": s.rev,
                })
            })
            .collect();
        crate::json::print_action_required(
            "confirm_migration",
            serde_json::json!({
                "skills": plan,
                "skipped": skipped,
            }),
        );
    }

    if resolved.is_empty() && !json {
        println!("No skills to migrate (all skipped).");
        print_skipped(&skipped);
        return Ok(());
    }

    // ── Phase 3: Dry run ──────────────────────────────────────────────────
    if dry_run {
        if !json {
            println!();
            println!("Dry run — would migrate {} skills:", resolved.len());
            for skill in &resolved {
                let source_display = format_source(&skill.source);
                let rev_display = skill
                    .rev
                    .as_deref()
                    .map(|r| format!(" @ {r}"))
                    .unwrap_or_default();
                println!("  {} ({}{}) ...", skill.name, source_display, rev_display);
            }
            println!();
            println!("Dry run complete. No files were written.");
            print_skipped(&skipped);
        } else {
            crate::json::print_success(serde_json::json!({
                "dry_run": true,
                "would_migrate": resolved.iter().map(|s| &s.name).collect::<Vec<_>>(),
                "skipped": skipped,
            }));
        }
        return Ok(());
    }

    // ── Phase 4: Execute migration ────────────────────────────────────────
    if !json {
        println!();
        println!("Migrating {} skills...", resolved.len());
    }

    let options = MigrateOptions {
        dry_run: false,
        manifest_options: merged_options.clone(),
    };

    let locked = ion_skill::migrate::migrate(project_dir, &resolved, &options)?;

    // ── Phase 5: Gitignore + registry ─────────────────────────────────────
    let target_paths: Vec<&str> = merged_options
        .targets
        .values()
        .map(|s| s.as_str())
        .collect();

    for entry in &locked {
        // Add per-skill gitignore entries
        ion_skill::gitignore::add_skill_entries(
            project_dir,
            &entry.name,
            &target_paths,
            merged_options.skills_dir_or_default(),
        )?;

        // Register in global registry
        if let Some(resolved_skill) = resolved.iter().find(|r| r.name == entry.name) {
            register_in_registry(&resolved_skill.source, project_dir)?;
        }

        if !json {
            let commit_display = entry
                .commit()
                .map(|c: &str| if c.len() > 7 { &c[..7] } else { c })
                .unwrap_or("(none)");
            println!(
                "  {} ... installed, commit {}",
                p.bold(&entry.name),
                p.dim(commit_display)
            );
        }
    }

    if !json && !locked.is_empty() {
        println!("  Updated {}", p.dim(".gitignore"));
        println!("  Updated {}", p.dim("Ion.toml"));
        println!("  Updated {}", p.dim("Ion.lock"));
    }

    // ── Phase 6: Discover leftover skills ─────────────────────────────────
    let migrated_names: HashSet<String> = locked.iter().map(|s| s.name.clone()).collect();
    let target_path_strs: Vec<String> = merged_options.targets.values().cloned().collect();
    let leftovers = discover_leftover_skills(project_dir, &migrated_names, &target_path_strs)?;

    let mut matched_skills: Vec<serde_json::Value> = Vec::new();
    let mut custom_skills: Vec<serde_json::Value> = Vec::new();

    if !leftovers.is_empty() {
        if !json {
            println!();
            println!(
                "Found {} leftover skill(s) not in lockfile:",
                leftovers.len()
            );
            for leftover in &leftovers {
                println!("  - {} ({})", leftover.name, origin_label(&leftover.origin));
            }
        }

        // ── Phase 7: Search for leftover matches ──────────────────────────
        let search_sources = crate::commands::search::build_sources(&ctx.global_config);
        let cache = ion_skill::search::SearchCache::new();
        let max_age_secs = ctx
            .global_config
            .cache
            .max_age_days
            .map(|d| u64::from(d) * 86400)
            .unwrap_or(86400);

        let mut lockfile = ctx.lockfile()?;

        for leftover in &leftovers {
            // Search for this skill by name
            let search_results = search_for_skill(
                &search_sources,
                &leftover.name,
                cache.as_ref(),
                max_age_secs,
            );

            let exact_match = search_results
                .iter()
                .find(|r| ion_skill::search::skill_dir_name(&r.source) == leftover.name);

            if let Some(matched) = exact_match {
                let source_str = &matched.source;
                if !json {
                    println!();
                    println!(
                        "  Found match for '{}': {}",
                        p.bold(&leftover.name),
                        p.info(source_str)
                    );
                }

                let should_switch = if yes || json {
                    true
                } else {
                    print!(
                        "  Switch '{}' to ion-managed from {}? [Y/n] ",
                        leftover.name, source_str
                    );
                    io::stdout().flush()?;
                    let mut answer = String::new();
                    io::stdin().read_line(&mut answer)?;
                    let answer = answer.trim();
                    answer.is_empty()
                        || answer.eq_ignore_ascii_case("y")
                        || answer.eq_ignore_ascii_case("yes")
                };

                if should_switch {
                    match ion_skill::source::SkillSource::infer(source_str) {
                        Ok(source) => {
                            let installer = ctx.installer(&merged_options);
                            let validation = InstallValidationOptions {
                                skip_validation: false,
                                allow_warnings: true,
                            };
                            match installer.install_with_options(
                                &leftover.name,
                                &source,
                                validation,
                            ) {
                                Ok(entry) => {
                                    ion_skill::manifest_writer::add_skill(
                                        &ctx.manifest_path,
                                        &leftover.name,
                                        &source,
                                    )?;
                                    ion_skill::gitignore::add_skill_entries(
                                        project_dir,
                                        &leftover.name,
                                        &target_paths,
                                        merged_options.skills_dir_or_default(),
                                    )?;
                                    register_in_registry(&source, project_dir)?;
                                    lockfile.upsert(entry);

                                    if !json {
                                        println!(
                                            "    {} switched to ion-managed",
                                            p.success(&leftover.name)
                                        );
                                    }
                                    matched_skills.push(serde_json::json!({
                                        "name": leftover.name,
                                        "source": source_str,
                                    }));
                                }
                                Err(e) => {
                                    if !json {
                                        eprintln!("    Failed to install '{}': {e}", leftover.name);
                                    }
                                    // Fall through to treat as custom
                                    handle_custom_skill(
                                        project_dir,
                                        leftover,
                                        &merged_options,
                                        &p,
                                        json,
                                        &mut custom_skills,
                                    )?;
                                }
                            }
                        }
                        Err(e) => {
                            if !json {
                                eprintln!("    Invalid source '{}': {e}", source_str);
                            }
                            handle_custom_skill(
                                project_dir,
                                leftover,
                                &merged_options,
                                &p,
                                json,
                                &mut custom_skills,
                            )?;
                        }
                    }
                } else {
                    handle_custom_skill(
                        project_dir,
                        leftover,
                        &merged_options,
                        &p,
                        json,
                        &mut custom_skills,
                    )?;
                }
            } else {
                // No match found — treat as custom project skill
                handle_custom_skill(
                    project_dir,
                    leftover,
                    &merged_options,
                    &p,
                    json,
                    &mut custom_skills,
                )?;
            }
        }

        lockfile.write_to(&ctx.lockfile_path)?;
    }

    // ── Phase 8: Ensure built-in ion-cli skill ────────────────────────────
    ctx.ensure_builtin_skill(&merged_options);

    // ── Phase 9: Git commit ───────────────────────────────────────────────
    let commit_hash = create_migration_commit(project_dir);

    // ── Output ────────────────────────────────────────────────────────────
    if json {
        crate::json::print_success(serde_json::json!({
            "migrated": locked.iter().map(|s| serde_json::json!({
                "name": s.name,
                "source": s.source,
                "commit": s.commit(),
            })).collect::<Vec<_>>(),
            "matched": matched_skills,
            "custom": custom_skills,
            "skipped": skipped,
            "gitignore_updated": true,
            "git_commit": commit_hash,
        }));
        return Ok(());
    }

    print_skipped(&skipped);

    if let Some(hash) = &commit_hash {
        println!();
        println!(
            "  Created git commit: {}",
            p.info(&hash[..hash.len().min(7)])
        );
    }

    println!();
    println!(
        "{}",
        p.success("Migration complete! You can now use `ion add`, `ion update`, etc.")
    );
    Ok(())
}

fn handle_custom_skill(
    project_dir: &std::path::Path,
    leftover: &DiscoveredSkill,
    options: &ManifestOptions,
    p: &Paint,
    json: bool,
    custom_skills: &mut Vec<serde_json::Value>,
) -> anyhow::Result<()> {
    move_skill_to_local(project_dir, leftover, options)?;

    // Add to manifest as a local skill
    let local_source = ion_skill::source::SkillSource::local();
    let manifest_path = project_dir.join("Ion.toml");
    ion_skill::manifest_writer::add_skill(&manifest_path, &leftover.name, &local_source)?;

    if !json {
        println!(
            "    {} moved to .agents/skills/ as local skill",
            p.dim(&leftover.name)
        );
    }

    custom_skills.push(serde_json::json!({
        "name": leftover.name,
        "moved_to": format!(".agents/skills/{}", leftover.name),
    }));

    Ok(())
}

fn search_for_skill(
    sources: &[Box<dyn SearchSource + Send>],
    skill_name: &str,
    cache: Option<&SearchCache>,
    _max_age_secs: u64,
) -> Vec<ion_skill::search::SearchResult> {
    // We can't use parallel_search because sources isn't owned.
    // Do a simple sequential search instead.
    let mut all_results = Vec::new();
    for source in sources {
        match source.search(skill_name, 10) {
            Ok(results) => {
                // Cache results if applicable
                if source.name() != "agent"
                    && let Some(c) = cache
                {
                    c.put(source.name(), skill_name, &results);
                }
                all_results.extend(results);
            }
            Err(e) => {
                log::debug!(
                    "search source '{}' failed for '{}': {e}",
                    source.name(),
                    skill_name
                );
            }
        }
    }
    all_results
}

fn create_migration_commit(project_dir: &std::path::Path) -> Option<String> {
    // Stage all ion-related files that exist
    let candidates = ["Ion.toml", "Ion.lock", ".gitignore", ".agents/"];
    let files_to_stage: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|f| {
            let path = project_dir.join(f);
            path.exists() || path.is_symlink()
        })
        .collect();

    let repo = ionem_shell::git::repo(project_dir);

    if !files_to_stage.is_empty() {
        let _ = repo.stage_files(&files_to_stage);
    }

    // Check if there are any staged changes
    let has_changes = repo.has_staged_changes().ok()?;
    if !has_changes {
        return None; // No changes to commit or not a git repo
    }

    // There are staged changes — commit them and return the SHA
    repo.create_commit(
        "chore: migrate skills to ion management\n\nMigrated from skills-lock.json to Ion.toml/Ion.lock.\nSkill directories are now symlinks to ion-managed global storage.\nLeftover skills handled as local or matched to remote sources.",
    )
    .ok()
}

fn resolve_skill(skill: &DiscoveredSkill, stdin: &mut impl BufRead) -> Option<ResolvedSkill> {
    let source = match &skill.source {
        Some(s) => s.clone(),
        None => {
            // Prompt for source
            print!(
                "Skill '{}' found in {} but source is unknown.\nEnter source (e.g., owner/repo/skill or git URL), or press Enter to skip:\n> ",
                skill.name,
                origin_label(&skill.origin),
            );
            io::stdout().flush().ok();

            let mut input = String::new();
            stdin.read_line(&mut input).ok()?;
            let input = input.trim();

            if input.is_empty() {
                return None;
            }

            match ion_skill::source::SkillSource::infer(input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("  Invalid source: {e}. Skipping '{}'.", skill.name);
                    return None;
                }
            }
        }
    };

    // Prompt for rev pinning
    print!(
        "Pin '{}' to a specific ref? (branch/tag/SHA, or Enter to use latest):\n> ",
        skill.name,
    );
    io::stdout().flush().ok();

    let mut rev_input = String::new();
    stdin.read_line(&mut rev_input).ok();
    let rev = rev_input.trim();
    let rev = if rev.is_empty() {
        None
    } else {
        Some(rev.to_string())
    };

    Some(ResolvedSkill {
        name: skill.name.clone(),
        source,
        rev,
    })
}

fn origin_label(origin: &DiscoveryOrigin) -> &'static str {
    match origin {
        DiscoveryOrigin::LockFile => "skills-lock.json",
        DiscoveryOrigin::AgentsDir => ".agents/skills/",
        DiscoveryOrigin::ClaudeDir => ".claude/skills/",
    }
}

fn format_source(source: &ion_skill::source::SkillSource) -> String {
    match &source.path {
        Some(path) => format!("{}/{}", source.source, path),
        None => source.source.clone(),
    }
}

fn print_skipped(skipped: &[String]) {
    if !skipped.is_empty() {
        println!();
        println!(
            "Skipped {} skills (add manually with `ion add`):",
            skipped.len()
        );
        for name in skipped {
            println!("  - {name}");
        }
    }
}
