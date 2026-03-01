use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;

pub fn run() -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory");
    }

    let manifest = Manifest::from_file(&manifest_path)?;
    let mut lockfile = Lockfile::from_file(&lockfile_path)?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Installing {} skill(s)...", manifest.skills.len());

    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing '{name}'...");
        let locked = install_skill(&project_dir, name, &source, &manifest.options)?;
        lockfile.upsert(locked);
    }

    lockfile.write_to(&lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");
    Ok(())
}
