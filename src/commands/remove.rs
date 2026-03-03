use ion_skill::installer::{SkillInstaller, hash_simple};
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::registry::Registry;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;

pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let manifest = ctx.manifest()?;

    let entry = manifest.skills.get(name).ok_or_else(|| {
        anyhow::anyhow!("Skill '{name}' not found in ion.toml")
    })?;

    let merged_options = ctx.merged_options(&manifest);

    println!("Removing skill '{name}'...");

    SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(name)?;
    println!("  Removed from .agents/skills/{name}/");

    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
    println!("  Updated .gitignore");

    // Unregister from global registry for git-based sources
    if let Ok(source) = Manifest::resolve_entry(entry) {
        if matches!(source.source_type, SourceType::Github | SourceType::Git) {
            if let Ok(url) = source.git_url() {
                let repo_hash = format!("{:x}", hash_simple(&url));
                let project_str = ctx.project_dir.display().to_string();
                let mut registry = Registry::load()?;
                registry.unregister(&repo_hash, &project_str);
                registry.save()?;
            }
        }
    }

    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.remove(name);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
