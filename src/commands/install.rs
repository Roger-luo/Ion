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

    // Check gitignore for managed directories
    let mut managed_dirs = vec![".agents/".to_string()];
    for path in manifest.options.targets.values() {
        let top_level = path.split('/').next().unwrap_or(path);
        let entry = format!("{top_level}/");
        if !managed_dirs.contains(&entry) {
            managed_dirs.push(entry);
        }
    }

    let dir_refs: Vec<&str> = managed_dirs.iter().map(|s| s.as_str()).collect();
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&project_dir, &dir_refs)?;

    if !missing.is_empty() {
        println!("\nThese directories are not in .gitignore:");
        for dir in &missing {
            println!("  {dir}");
        }
        print!("\nAdd them? [y/n] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;

        if answer.trim().eq_ignore_ascii_case("y") {
            let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
            ion_skill::gitignore::append_to_gitignore(&project_dir, &refs)?;
            println!("Updated .gitignore");
        }
    }

    Ok(())
}
