use ion_skill::installer::{SkillInstaller, hash_simple};
use ion_skill::manifest_writer;
use ion_skill::registry::Registry;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: &str, yes: bool, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;

    // If the argument matches a skill name exactly, remove that single skill.
    // Otherwise, fuzzy-match against skill names and source strings.
    let skills_to_remove: Vec<String> = if manifest.skills.contains_key(name) {
        vec![name.to_string()]
    } else {
        let matches: Vec<String> = manifest
            .skills
            .iter()
            .filter(|(skill_name, entry)| skill_matches(skill_name, entry, name))
            .map(|(skill_name, _)| skill_name.clone())
            .collect();
        if matches.is_empty() {
            anyhow::bail!("No skills matching '{name}' found in Ion.toml");
        }
        matches
    };

    // Confirm before removing
    if !json {
        println!(
            "Will remove {} skill(s):",
            p.bold(&skills_to_remove.len().to_string())
        );
        for skill_name in &skills_to_remove {
            println!("  - {}", p.bold(skill_name));
        }
    }

    if json && !yes {
        crate::json::print_action_required(
            "confirm_removal",
            serde_json::json!({
                "skills": skills_to_remove,
            }),
        );
    }

    if !json && !yes {
        use std::io::Write;
        print!("Proceed? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") && !answer.trim().eq_ignore_ascii_case("yes") {
            anyhow::bail!("Aborted.");
        }
    }

    let merged_options = ctx.merged_options(&manifest);
    let mut lockfile = ctx.lockfile()?;

    for skill_name in &skills_to_remove {
        let entry = &manifest.skills[skill_name];
        let entry_source = entry.resolve();

        if !json {
            println!("Removing skill {}...", p.bold(&format!("'{skill_name}'")));
        }

        // For local skills, only remove symlinks — preserve the actual skill directory
        if let Ok(ref source) = entry_source
            && source.source_type == SourceType::Local
        {
            for target_path in merged_options.targets.values() {
                let target_dir = ctx.project_dir.join(target_path).join(skill_name);
                if target_dir.is_symlink() {
                    std::fs::remove_file(&target_dir)?;
                }
            }
            // Remove .agents symlink only if it IS a symlink (custom skills-dir)
            let agents_dir = ctx
                .project_dir
                .join(".agents")
                .join("skills")
                .join(skill_name);
            if agents_dir.is_symlink() {
                std::fs::remove_file(&agents_dir)?;
            }
            if !json {
                println!("  Removed symlinks for {}", p.info(skill_name));
                println!("  {}: local skill directory preserved", p.dim("note"));
            }
        } else {
            SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(skill_name)?;
            if !json {
                println!(
                    "  Removed from {}",
                    p.info(&format!(".agents/skills/{skill_name}/"))
                );
            }
        }

        // Skip gitignore removal for local skills (they were never gitignored)
        if !matches!(
            entry_source.as_ref().map(|s| &s.source_type),
            Ok(SourceType::Local)
        ) {
            ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, skill_name)?;
            if !json {
                println!("  Updated {}", p.dim(".gitignore"));
            }
        }

        // Unregister from global registry for git-based sources
        if let Ok(ref source) = entry_source
            && matches!(source.source_type, SourceType::Github | SourceType::Git)
            && let Ok(url) = source.git_url()
        {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = ctx.project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.unregister(&repo_hash, &project_str);
            registry.save()?;
        }

        // Clean up binary files if this is a binary skill
        if let Some(locked) = lockfile.find(skill_name) {
            if let Some(ref binary_name) = locked.binary {
                if let Some(ref version) = locked.binary_version {
                    ion_skill::binary::remove_binary_version(binary_name, version)?;
                    if !json {
                        println!(
                            "  Removed binary {} v{}",
                            p.info(binary_name),
                            p.dim(version)
                        );
                    }
                }
            }
        }

        manifest_writer::remove_skill(&ctx.manifest_path, skill_name)?;
        lockfile.remove(skill_name);
    }

    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "removed": skills_to_remove,
        }));
        return Ok(());
    }

    println!("  Updated {}", p.dim("Ion.toml"));
    println!("  Updated {}", p.dim("Ion.lock"));
    println!("{}", p.success("Done!"));
    Ok(())
}

/// Check if a skill matches the query by name or source.
/// Matches if the query appears anywhere in the skill name or source string.
fn skill_matches(skill_name: &str, entry: &ion_skill::manifest::SkillEntry, query: &str) -> bool {
    let source_str = match entry {
        ion_skill::manifest::SkillEntry::Shorthand(s) => Some(s.as_str()),
        ion_skill::manifest::SkillEntry::Full { source, .. } => source.as_deref(),
    };
    skill_name.contains(query) || source_str.is_some_and(|s| s.contains(query))
}
