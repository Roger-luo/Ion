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
    let lockfile = Lockfile::from_file(&lockfile_path)?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Skills ({}):", manifest.skills.len());
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        let locked = lockfile.find(name);

        let version_str = locked
            .and_then(|l| l.version.as_deref())
            .unwrap_or("unknown");
        let commit_str = locked
            .and_then(|l| l.commit.as_deref())
            .map(|c| &c[..c.len().min(8)])
            .unwrap_or("none");
        let installed = project_dir
            .join(".agents")
            .join("skills")
            .join(name)
            .exists();
        let status = if installed {
            "installed"
        } else {
            "not installed"
        };

        println!("  {name} v{version_str} ({commit_str}) [{status}]");
        println!("    source: {}", source.source);
    }
    Ok(())
}
