use ion_skill::lockfile::LockedSkill;
use ion_skill::source::SkillSourceKind;
use ion_skill::update::Updater;
use ion_skill::update::binary::BinaryUpdater;
use ion_skill::update::git::GitUpdater;

use crate::context::ProjectContext;

pub fn run(name: Option<&str>, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = ctx.paint();
    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    let options = ctx.merged_options(&manifest);
    let installer = ctx.installer(&options);

    let skills_to_check: Vec<(String, _)> = manifest
        .skills
        .iter()
        .filter(|(skill_name, _)| name.is_none() || name == Some(skill_name.as_str()))
        .filter_map(|(skill_name, entry)| match entry.resolve() {
            Ok(source) => Some((skill_name.clone(), source)),
            Err(e) => {
                eprintln!("Warning: skipping '{}': {}", skill_name, e);
                None
            }
        })
        .collect();

    if skills_to_check.is_empty() {
        if let Some(n) = name {
            anyhow::bail!("No skill '{}' found in Ion.toml", n);
        }
        if json {
            crate::json::print_success(serde_json::json!({
                "updated": [],
                "skipped": [],
                "failed": [],
                "up_to_date": [],
            }));
            return Ok(());
        }
        println!("No skills to update.");
        return Ok(());
    }

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

        // Skip non-binary skills with rev set (pinned)
        if !source.is_binary()
            && let Some(ref rev) = source.rev
        {
            if !json {
                println!(
                    "  {} {}  {}",
                    p.dim("-"),
                    p.bold(skill_name),
                    p.dim(&format!("skipped (pinned to {})", rev))
                );
            }
            json_skipped.push(
                serde_json::json!({ "name": skill_name, "reason": format!("pinned to {}", rev) }),
            );
            skipped_count += 1;
            continue;
        }

        // Select updater based on source type
        let updater: Box<dyn Updater> = match &source.kind {
            SkillSourceKind::Binary { .. } => Box::new(BinaryUpdater),
            SkillSourceKind::Github | SkillSourceKind::Git => Box::new(GitUpdater),
            _ => continue,
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
        let info = match updater.check(&locked, source) {
            Ok(Some(info)) => info,
            Ok(None) => {
                if !json {
                    println!(
                        "  {} {}  {}",
                        p.dim("·"),
                        p.bold(skill_name),
                        p.dim("already up to date")
                    );
                }
                json_up_to_date.push(serde_json::json!({ "name": skill_name }));
                up_to_date_count += 1;
                continue;
            }
            Err(e) => {
                if !json {
                    println!(
                        "  {} {}  {}",
                        p.warn("✗"),
                        p.bold(skill_name),
                        p.warn(&format!("check failed: {}", e))
                    );
                }
                json_failed.push(serde_json::json!({ "name": skill_name, "error": e.to_string() }));
                failed_count += 1;
                continue;
            }
        };

        // Apply update
        match updater.apply(&locked, source, &installer) {
            Ok(new_locked) => {
                if !json {
                    let binary_suffix = if source.is_binary() { " (binary)" } else { "" };
                    println!(
                        "  {} {}  {} → {}{}",
                        p.success("✓"),
                        p.bold(skill_name),
                        info.old_version,
                        p.info(&info.new_version),
                        binary_suffix
                    );
                }
                json_updated.push(serde_json::json!({
                    "name": skill_name,
                    "old_version": info.old_version,
                    "new_version": info.new_version,
                    "binary": source.is_binary(),
                }));
                lockfile.upsert(new_locked);
                updated_count += 1;
            }
            Err(e) => {
                if !json {
                    println!(
                        "  {} {}  {}",
                        p.warn("✗"),
                        p.bold(skill_name),
                        p.warn(&format!("{}", e))
                    );
                }
                json_failed.push(serde_json::json!({ "name": skill_name, "error": e.to_string() }));
                failed_count += 1;
            }
        }
    }

    // Write lockfile only if something changed
    if updated_count > 0 {
        lockfile.write_to(&ctx.lockfile_path)?;
    }

    // Check for agents template update (non-fatal)
    if manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .is_some()
        && let Err(e) =
            crate::commands::agents::update_template_non_fatal(&ctx, &mut lockfile, &p, json)
        && !json
    {
        println!(
            "  {} agents template: {}",
            p.warn("⚠"),
            p.warn(&e.to_string())
        );
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": json_updated,
            "skipped": json_skipped,
            "failed": json_failed,
            "up_to_date": json_up_to_date,
        }));
        return Ok(());
    }

    // Print summary
    let mut parts = Vec::new();
    if updated_count > 0 {
        parts.push(format!("{} updated", updated_count));
    }
    if skipped_count > 0 {
        parts.push(format!("{} skipped", skipped_count));
    }
    if failed_count > 0 {
        parts.push(format!("{} failed", failed_count));
    }
    if up_to_date_count > 0 {
        parts.push(format!("{} up to date", up_to_date_count));
    }
    if !parts.is_empty() {
        println!("\n{}", parts.join(", "));
    }

    Ok(())
}
