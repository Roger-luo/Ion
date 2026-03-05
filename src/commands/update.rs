use ion_skill::lockfile::LockedSkill;
use ion_skill::manifest::Manifest;
use ion_skill::source::SourceType;
use ion_skill::update::binary::BinaryUpdater;
use ion_skill::update::git::GitUpdater;
use ion_skill::update::{UpdateContext, Updater};

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    let options = ctx.merged_options(&manifest);
    let update_ctx = UpdateContext {
        project_dir: &ctx.project_dir,
        options: &options,
    };

    let skills_to_check: Vec<(String, _)> = manifest
        .skills
        .iter()
        .filter(|(skill_name, _)| name.is_none() || name == Some(skill_name.as_str()))
        .filter_map(|(skill_name, entry)| {
            let source = Manifest::resolve_entry(entry).ok()?;
            Some((skill_name.clone(), source))
        })
        .collect();

    if skills_to_check.is_empty() {
        if let Some(n) = name {
            anyhow::bail!("No skill '{}' found in Ion.toml", n);
        }
        println!("No skills to update.");
        return Ok(());
    }

    let mut updated_count = 0u32;
    let mut skipped_count = 0u32;
    let mut failed_count = 0u32;
    let mut up_to_date_count = 0u32;

    for (skill_name, source) in &skills_to_check {
        // Skip Path and Http source types silently
        if matches!(source.source_type, SourceType::Path | SourceType::Http) {
            continue;
        }

        // Skip non-binary skills with rev set (pinned)
        if source.source_type != SourceType::Binary
            && let Some(ref rev) = source.rev
        {
            println!(
                "  {} {}  {}",
                p.dim("-"),
                p.bold(skill_name),
                p.dim(&format!("skipped (pinned to {})", rev))
            );
            skipped_count += 1;
            continue;
        }

        // Select updater based on source type
        let updater: Box<dyn Updater> = match source.source_type {
            SourceType::Binary => Box::new(BinaryUpdater),
            SourceType::Github | SourceType::Git => Box::new(GitUpdater),
            _ => continue,
        };

        // Get or create locked skill
        let locked = lockfile
            .find(skill_name)
            .cloned()
            .unwrap_or_else(|| LockedSkill {
                name: skill_name.clone(),
                source: source.source.clone(),
                path: source.path.clone(),
                version: None,
                commit: None,
                checksum: None,
                binary: None,
                binary_version: None,
                binary_checksum: None,
            });

        // Check for update
        let info = match updater.check(&locked, source) {
            Ok(Some(info)) => info,
            Ok(None) => {
                println!(
                    "  {} {}  {}",
                    p.dim("·"),
                    p.bold(skill_name),
                    p.dim("already up to date")
                );
                up_to_date_count += 1;
                continue;
            }
            Err(e) => {
                println!(
                    "  {} {}  {}",
                    p.warn("✗"),
                    p.bold(skill_name),
                    p.warn(&format!("check failed: {}", e))
                );
                failed_count += 1;
                continue;
            }
        };

        // Apply update
        match updater.apply(&locked, source, &update_ctx) {
            Ok(new_locked) => {
                let binary_suffix = if source.source_type == SourceType::Binary {
                    " (binary)"
                } else {
                    ""
                };
                println!(
                    "  {} {}  {} → {}{}",
                    p.success("✓"),
                    p.bold(skill_name),
                    info.old_version,
                    p.info(&info.new_version),
                    binary_suffix
                );
                lockfile.upsert(new_locked);
                updated_count += 1;
            }
            Err(e) => {
                println!(
                    "  {} {}  {}",
                    p.warn("✗"),
                    p.bold(skill_name),
                    p.warn(&format!("{}", e))
                );
                failed_count += 1;
            }
        }
    }

    // Write lockfile only if something changed
    if updated_count > 0 {
        lockfile.write_to(&ctx.lockfile_path)?;
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
