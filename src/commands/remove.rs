use ion_skill::config::GlobalConfig;
use ion_skill::installer::uninstall_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};
use ion_skill::manifest_writer;

pub fn run(name: &str) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    let manifest = Manifest::from_file(&manifest_path)?;
    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    let global_config = GlobalConfig::load()?;
    let merged_targets = global_config.resolve_targets(&manifest.options);
    let merged_options = ManifestOptions { targets: merged_targets };

    println!("Removing skill '{name}'...");

    uninstall_skill(&project_dir, name, &merged_options)?;
    println!("  Removed from .agents/skills/{name}/");

    manifest_writer::remove_skill(&manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.remove(name);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
