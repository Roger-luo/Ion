use std::path::Path;

use ion_skill::installer::{SkillInstaller, builtin_skills_dir};
use ion_skill::manifest::ManifestOptions;
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

const SKILL_NAME: &str = "ion-cli";
const SKILL_CONTENT: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));

/// Update the global SKILL.md if the embedded content has changed.
///
/// Called on every ion invocation so that after `ion self update`, the next
/// command automatically refreshes the global copy. Since all projects
/// symlink to this global file, every project sees the update immediately.
///
/// This is a cheap operation: one file read + string compare, no-op if current.
pub fn refresh_global() {
    let global_dir = builtin_skills_dir().join(SKILL_NAME);
    let global_skill_md = global_dir.join("SKILL.md");

    let needs_write = if global_skill_md.exists() {
        std::fs::read_to_string(&global_skill_md).ok().as_deref() != Some(SKILL_CONTENT)
    } else {
        // Don't create the global dir on first run — wait for `ensure_installed`
        // which also sets up symlinks and Ion.toml registration.
        return;
    };

    if needs_write {
        let _ = std::fs::write(&global_skill_md, SKILL_CONTENT);
    }
}

/// Ensure the ion-cli built-in skill is installed in the project.
///
/// Writes the embedded SKILL.md to global storage if needed,
/// deploys symlinks into the project, and registers in Ion.toml.
pub fn ensure_installed(
    project_dir: &Path,
    manifest_path: &Path,
    options: &ManifestOptions,
) -> anyhow::Result<()> {
    let global_dir = builtin_skills_dir().join(SKILL_NAME);
    let global_skill_md = global_dir.join("SKILL.md");

    // Write/update SKILL.md in global storage
    let needs_write = if global_skill_md.exists() {
        std::fs::read_to_string(&global_skill_md).ok().as_deref() != Some(SKILL_CONTENT)
    } else {
        true
    };

    if needs_write {
        std::fs::create_dir_all(&global_dir)?;
        std::fs::write(&global_skill_md, SKILL_CONTENT)?;
    }

    // Deploy symlinks: global → .agents/skills/ion-cli → targets
    let installer = SkillInstaller::new(project_dir, options);
    installer.deploy(SKILL_NAME, &global_dir)?;

    // Gitignore the symlinks (they point to global storage, not project-local content)
    let target_paths: Vec<&str> = options.targets.values().map(|s| s.as_str()).collect();
    ion_skill::gitignore::add_skill_entries(project_dir, SKILL_NAME, &target_paths)?;

    // Register as local skill in Ion.toml if not already present
    let content = std::fs::read_to_string(manifest_path).unwrap_or_default();
    if !content.contains(&format!("[skills.{SKILL_NAME}]"))
        && !content.contains(&format!("{SKILL_NAME} ="))
        && !content.contains(&format!("\"{SKILL_NAME}\""))
    {
        let source = SkillSource::local();
        manifest_writer::add_skill(manifest_path, SKILL_NAME, &source)?;
    }

    Ok(())
}
