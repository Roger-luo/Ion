use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    let mut source = SkillSource::infer(source_str)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

    let manifest = if manifest_path.exists() {
        Manifest::from_file(&manifest_path)?
    } else {
        Manifest::empty()
    };

    let locked = install_skill(&project_dir, &name, &source, &manifest.options)?;
    println!("  Installed to .agents/skills/{name}/");
    for target_name in manifest.options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    manifest_writer::add_skill(&manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.upsert(locked);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}

fn skill_name_from_source(source: &SkillSource) -> String {
    if let Some(ref path) = source.path {
        path.rsplit('/').next().unwrap_or(path).to_string()
    } else {
        source
            .source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&source.source)
            .to_string()
    }
}
