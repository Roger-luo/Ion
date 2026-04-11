use std::io::IsTerminal;

use indicatif::{ProgressBar, ProgressStyle};
use ion_skill::lockfile::LockedSkill;
use ion_skill::source::SkillSourceKind;
use ion_skill::update::Updater;
use ion_skill::update::binary::BinaryUpdater;
use ion_skill::update::git::GitUpdater;
use ion_skill::workspace::Project;

use crate::context::WorkspaceContext;
use crate::style::Paint;

pub fn run(name: Option<&str>, json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let projects = ws.scoped_projects();
    let p = ws.paint();
    let multi = projects.len() > 1;

    let mut total_updated = 0u32;
    let mut total_skipped = 0u32;
    let mut total_failed = 0u32;
    let mut total_up_to_date = 0u32;

    let mut all_json_updated: Vec<serde_json::Value> = Vec::new();
    let mut all_json_skipped: Vec<serde_json::Value> = Vec::new();
    let mut all_json_failed: Vec<serde_json::Value> = Vec::new();
    let mut all_json_up_to_date: Vec<serde_json::Value> = Vec::new();

    for project in &projects {
        if !project.has_manifest() {
            continue;
        }
        let manifest = project.manifest()?;
        let mut lockfile = project.lockfile()?;

        let options = ws.merged_options_for(project)?;
        let installer = ws.installer_for(project, &options);

        // Ensure built-in skill and agent symlinks are up to date (non-fatal)
        ws.ensure_builtin_skill(project, &options);
        if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&project.dir, &options.targets) {
            log::warn!("Failed to create agent symlinks: {e}");
        }

        if multi && !json {
            let label = project_label(project, &ws);
            println!("\n{}:", p.bold(&label));
        }

        let (updated, skipped, failed, up_to_date, ju, js, jf, jut) = update_project_skills(
            name,
            json,
            &manifest,
            &mut lockfile,
            &installer,
            &p,
            project,
        )?;

        total_updated += updated;
        total_skipped += skipped;
        total_failed += failed;
        total_up_to_date += up_to_date;
        all_json_updated.extend(ju);
        all_json_skipped.extend(js);
        all_json_failed.extend(jf);
        all_json_up_to_date.extend(jut);

        // Check for agents template update (non-fatal)
        let mut agents_updated = false;
        if manifest
            .agents
            .as_ref()
            .and_then(|a| a.template.as_ref())
            .is_some()
        {
            match crate::commands::agents::update_template_non_fatal(
                project,
                &ws.global_config,
                &mut lockfile,
                &p,
                json,
            ) {
                Ok(()) => agents_updated = true,
                Err(e) if !json => {
                    println!(
                        "  {} agents template: {}",
                        p.warn("⚠"),
                        p.warn(&e.to_string())
                    );
                }
                Err(_) => {}
            }
        }

        // Write lockfile if skills or agents template changed
        if updated > 0 || agents_updated {
            lockfile.write_to(&project.lockfile_path)?;
        }
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": all_json_updated,
            "skipped": all_json_skipped,
            "failed": all_json_failed,
            "up_to_date": all_json_up_to_date,
        }));
        return Ok(());
    }

    // Print summary
    let mut parts = Vec::new();
    if total_updated > 0 {
        parts.push(format!("{} updated", total_updated));
    }
    if total_skipped > 0 {
        parts.push(format!("{} skipped", total_skipped));
    }
    if total_failed > 0 {
        parts.push(format!("{} failed", total_failed));
    }
    if total_up_to_date > 0 {
        parts.push(format!("{} up to date", total_up_to_date));
    }
    if !parts.is_empty() {
        println!("\n{}", parts.join(", "));
    } else if name.is_some() {
        // Only bail if targeting a specific skill and it wasn't found in any project
        let n = name.unwrap();
        anyhow::bail!("No skill '{}' found in Ion.toml", n);
    } else {
        println!("No skills to update.");
    }

    Ok(())
}

/// Update skills within a single project. Returns counts and JSON arrays.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn update_project_skills(
    name: Option<&str>,
    json: bool,
    manifest: &ion_skill::manifest::Manifest,
    lockfile: &mut ion_skill::lockfile::Lockfile,
    installer: &ion_skill::installer::SkillInstaller<'_>,
    p: &Paint,
    project: &Project,
) -> anyhow::Result<(
    u32,
    u32,
    u32,
    u32,
    Vec<serde_json::Value>,
    Vec<serde_json::Value>,
    Vec<serde_json::Value>,
    Vec<serde_json::Value>,
)> {
    let skills_to_check: Vec<(String, _)> = manifest
        .skills
        .iter()
        .filter(|(skill_name, _)| name.is_none() || name == Some(skill_name.as_str()))
        .filter_map(|(skill_name, entry)| match entry.resolve() {
            Ok(source) => Some((skill_name.clone(), source)),
            Err(e) => {
                eprintln!("warning: skipping '{}': {}", skill_name, e);
                None
            }
        })
        .collect();

    if skills_to_check.is_empty() {
        return Ok((0, 0, 0, 0, Vec::new(), Vec::new(), Vec::new(), Vec::new()));
    }

    // Count updatable skills (skip path/http/local which are silently ignored)
    let updatable_count = skills_to_check
        .iter()
        .filter(|(_, s)| !s.is_path() && !s.is_http() && !s.is_local())
        .count() as u64;

    let pb = if !json && updatable_count > 0 {
        Some(make_progress_bar(p, updatable_count))
    } else {
        None
    };

    let mut updated_count = 0u32;
    let mut skipped_count = 0u32;
    let mut failed_count = 0u32;
    let mut up_to_date_count = 0u32;

    let mut json_updated: Vec<serde_json::Value> = Vec::new();
    let mut json_skipped: Vec<serde_json::Value> = Vec::new();
    let mut json_failed: Vec<serde_json::Value> = Vec::new();
    let mut json_up_to_date: Vec<serde_json::Value> = Vec::new();

    for (skill_name, source) in &skills_to_check {
        // Skip Path and Http source types silently
        if source.is_path() || source.is_http() || source.is_local() {
            continue;
        }

        if let Some(ref pb) = pb {
            pb.set_message(format!("checking {}", skill_name));
        }

        // Skip non-binary skills with rev set (pinned)
        if !source.is_binary()
            && let Some(ref rev) = source.rev
        {
            if !json {
                pb_println(
                    &pb,
                    format!(
                        "  {} {}  {}",
                        p.dim("-"),
                        p.bold(skill_name),
                        p.dim(&format!("skipped (pinned to {})", rev))
                    ),
                );
            }
            json_skipped.push(
                serde_json::json!({ "name": skill_name, "reason": format!("pinned to {}", rev) }),
            );
            skipped_count += 1;
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
            continue;
        }

        // Select updater based on source type
        let updater: Box<dyn Updater> = match &source.kind {
            SkillSourceKind::Binary { .. } => Box::new(BinaryUpdater),
            SkillSourceKind::Github | SkillSourceKind::Git => Box::new(GitUpdater),
            _ => {
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                continue;
            }
        };

        // Get or create locked skill
        let locked = lockfile.find(skill_name).cloned().unwrap_or_else(|| {
            let mut fallback = if source.is_binary() {
                let binary_name = match &source.kind {
                    SkillSourceKind::Binary { binary_name, .. } if !binary_name.is_empty() => {
                        binary_name.as_str()
                    }
                    _ => skill_name.as_str(),
                };
                LockedSkill::binary(
                    skill_name.clone(),
                    source.source.clone(),
                    binary_name,
                    None,
                    None,
                )
            } else {
                LockedSkill::git(
                    skill_name.clone(),
                    source.source.clone(),
                    String::new(),
                    String::new(),
                )
            };
            if let Some(ref path) = source.path {
                fallback = fallback.with_path(path.clone());
            }
            fallback
        });

        // Check for update
        let update_info = match updater.check(&locked, source) {
            Ok(Some(info)) => Some(info),
            Ok(None) => {
                // Remote is up to date — verify local deployment is intact
                if installer.is_deployed(skill_name) {
                    if !json {
                        pb_println(
                            &pb,
                            format!(
                                "  {} {}  {}",
                                p.dim("·"),
                                p.bold(skill_name),
                                p.dim("already up to date")
                            ),
                        );
                    }
                    json_up_to_date.push(serde_json::json!({ "name": skill_name }));
                    up_to_date_count += 1;
                    if let Some(ref pb) = pb {
                        pb.inc(1);
                    }
                    continue;
                }
                // Local deployment is broken — needs repair
                None
            }
            Err(e) => {
                if !json {
                    pb_println(
                        &pb,
                        format!(
                            "  {} {}  {}",
                            p.warn("✗"),
                            p.bold(skill_name),
                            p.warn(&format!("check failed: {}", e))
                        ),
                    );
                }
                json_failed.push(serde_json::json!({ "name": skill_name, "error": e.to_string() }));
                failed_count += 1;
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                continue;
            }
        };

        // Apply update or repair
        if let Some(ref pb) = pb {
            let action = if update_info.is_some() {
                "updating"
            } else {
                "repairing"
            };
            pb.set_message(format!("{} {}", action, skill_name));
        }

        match updater.apply(&locked, source, installer) {
            Ok(new_locked) => {
                if !json {
                    if let Some(ref info) = update_info {
                        let binary_suffix = if source.is_binary() { " (binary)" } else { "" };
                        pb_println(
                            &pb,
                            format!(
                                "  {} {}  {} → {}{}",
                                p.success("✓"),
                                p.bold(skill_name),
                                info.old_version,
                                p.info(&info.new_version),
                                binary_suffix
                            ),
                        );
                    } else {
                        pb_println(
                            &pb,
                            format!(
                                "  {} {}  {}",
                                p.success("✓"),
                                p.bold(skill_name),
                                p.info("repaired")
                            ),
                        );
                    }
                }
                if let Some(ref info) = update_info {
                    json_updated.push(serde_json::json!({
                        "name": skill_name,
                        "old_version": info.old_version,
                        "new_version": info.new_version,
                        "binary": source.is_binary(),
                    }));
                } else {
                    json_updated.push(serde_json::json!({
                        "name": skill_name,
                        "repaired": true,
                    }));
                }
                lockfile.upsert(new_locked);
                updated_count += 1;
            }
            Err(e) => {
                if !json {
                    pb_println(
                        &pb,
                        format!(
                            "  {} {}  {}",
                            p.warn("✗"),
                            p.bold(skill_name),
                            p.warn(&format!("{}", e))
                        ),
                    );
                }
                json_failed.push(serde_json::json!({ "name": skill_name, "error": e.to_string() }));
                failed_count += 1;
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    // Finish the progress bar
    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    let _ = project; // used for scope clarity

    Ok((
        updated_count,
        skipped_count,
        failed_count,
        up_to_date_count,
        json_updated,
        json_skipped,
        json_failed,
        json_up_to_date,
    ))
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

/// Create a progress bar styled for the update command.
fn make_progress_bar(p: &Paint, total: u64) -> ProgressBar {
    let pb = if std::io::stderr().is_terminal() {
        ProgressBar::new(total)
    } else {
        ProgressBar::hidden()
    };

    if p.color {
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.cyan} [{bar:20.cyan/dim}] {pos}/{len}  {msg:.dim}")
                .unwrap()
                .progress_chars("━╸─"),
        );
    } else {
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  [{bar:20}] {pos}/{len}  {msg}")
                .unwrap()
                .progress_chars("=> "),
        );
    }

    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb
}

/// Print a line above the progress bar (if present), otherwise just println.
fn pb_println(pb: &Option<ProgressBar>, msg: String) {
    match pb {
        Some(pb) => pb.println(msg),
        None => println!("{}", msg),
    }
}
