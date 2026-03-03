use ion_skill::installer::SkillInstaller;
use ion_skill::manifest_writer;

use crate::context::ProjectContext;

pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let manifest = ctx.manifest()?;

    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Removing skill '{name}'...");

    SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(name)?;
    println!("  Removed from .agents/skills/{name}/");

    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
    println!("  Updated .gitignore");

    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.remove(name);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
