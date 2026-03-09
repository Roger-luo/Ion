use std::path::Path;

use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::{SkillSource, SourceType};

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: &str, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;

    // Verify skill exists and is not already local/path
    let entry = manifest
        .skills
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found in Ion.toml", name))?;
    let source = Manifest::resolve_entry(entry)?;

    if matches!(source.source_type, SourceType::Local | SourceType::Path) {
        anyhow::bail!("Skill '{}' is already local", name);
    }

    // Resolve skills-dir from merged options (default ".agents")
    let merged_options = ctx.merged_options(&manifest);
    let skills_dir = merged_options.skills_dir.as_deref().unwrap_or(".agents");

    // Find the current installed skill at .agents/skills/<name>
    let agents_skill = ctx.project_dir.join(".agents").join("skills").join(name);
    if !agents_skill.exists() {
        anyhow::bail!("Skill '{}' is not installed. Run `ion add` first.", name);
    }

    // Resolve the actual content by following symlinks
    let real_source = std::fs::canonicalize(&agents_skill)?;

    // Determine destination
    let dest = ctx.project_dir.join(skills_dir).join("skills").join(name);

    if !json {
        println!(
            "Ejecting skill {} to local copy...",
            p.bold(&format!("'{name}'"))
        );
    }

    // Handle the copy based on whether skills-dir is ".agents" or custom
    if skills_dir == ".agents" {
        // dest == agents_skill path. Remove the old symlink first, then copy content there.
        if agents_skill.is_symlink() {
            std::fs::remove_file(&agents_skill)?;
        } else if agents_skill.is_dir() {
            std::fs::remove_dir_all(&agents_skill)?;
        }
        copy_dir_recursive(&real_source, &dest)?;
    } else {
        // Custom skills-dir: copy to the custom location
        if dest.exists() {
            anyhow::bail!(
                "Destination already exists: {}. Remove it first.",
                dest.display()
            );
        }
        std::fs::create_dir_all(dest.parent().unwrap())?;
        copy_dir_recursive(&real_source, &dest)?;

        // Remove old .agents symlink and create new one pointing at the custom location
        if agents_skill.is_symlink() {
            std::fs::remove_file(&agents_skill)?;
        } else if agents_skill.is_dir() {
            std::fs::remove_dir_all(&agents_skill)?;
        }

        let rel_target = pathdiff::diff_paths(&dest, agents_skill.parent().unwrap())
            .ok_or_else(|| anyhow::anyhow!("Failed to compute relative path"))?;
        std::os::unix::fs::symlink(&rel_target, &agents_skill)?;
    }

    let display_dest = dest
        .strip_prefix(&ctx.project_dir)
        .unwrap_or(&dest)
        .display();
    if !json {
        println!(
            "  Copied skill content to {}",
            p.info(&display_dest.to_string())
        );
    }

    // Target symlinks (.claude/skills/<name> etc.) already point at .agents, no update needed

    // Update Ion.toml: remove old entry, add new local entry with forked-from
    let forked_from = build_forked_from(&source);

    let local_source = SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: Some(forked_from.clone()),
    };
    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    manifest_writer::add_skill(&ctx.manifest_path, name, &local_source)?;
    if !json {
        println!("  Updated {} — type changed to local", p.dim("Ion.toml"));
    }

    // Remove gitignore entries (local skills are tracked by git)
    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
    if !json {
        println!("  Updated {} — removed skill entries", p.dim(".gitignore"));
    }

    // Update lockfile: drop commit hash, keep checksum
    let mut lockfile = ctx.lockfile()?;
    if let Some(locked) = lockfile.find(name).cloned() {
        let updated = ion_skill::lockfile::LockedSkill {
            commit: None,
            ..locked
        };
        lockfile.upsert(updated);
        lockfile.write_to(&ctx.lockfile_path)?;
        if !json {
            println!("  Updated {}", p.dim("Ion.lock"));
        }
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": display_dest.to_string(),
            "forked_from": forked_from,
        }));
        return Ok(());
    }

    println!(
        "{} Ejected '{}' to {}",
        p.success("Done!"),
        name,
        display_dest
    );
    println!("  You can now edit the skill directly. Changes are tracked by git.");

    Ok(())
}

/// Build the forked-from string from the original source.
/// For GitHub skills with a path, format as "owner/repo/path".
/// For other types, use the source string directly.
fn build_forked_from(source: &SkillSource) -> String {
    if source.source_type == SourceType::Github
        && let Some(ref path) = source.path
    {
        return format!("{}/{}", source.source, path);
    }
    source.source.clone()
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
