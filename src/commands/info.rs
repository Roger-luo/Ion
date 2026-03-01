use ion_skill::manifest::Manifest;
use ion_skill::skill::SkillMetadata;
use ion_skill::source::SkillSource;

pub fn run(skill_str: &str) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");

    // First check if it's a name in the manifest
    if manifest_path.exists() {
        let manifest = Manifest::from_file(&manifest_path)?;
        if manifest.skills.contains_key(skill_str) {
            return show_info_from_installed(&project_dir, skill_str);
        }
    }

    // Otherwise try to resolve as a source
    let source = SkillSource::infer(skill_str)?;
    println!("Fetching info for '{skill_str}'...");
    println!("  Source type: {:?}", source.source_type);
    println!("  Source: {}", source.source);
    if let Some(ref path) = source.path {
        println!("  Path: {path}");
    }
    if let Ok(url) = source.git_url() {
        println!("  Git URL: {url}");
    }
    Ok(())
}

fn show_info_from_installed(project_dir: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let skill_md = project_dir
        .join(".agents")
        .join("skills")
        .join(name)
        .join("SKILL.md");

    if !skill_md.exists() {
        anyhow::bail!("Skill '{name}' is in ion.toml but not installed. Run `ion install`.");
    }

    let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

    println!("Skill: {}", meta.name);
    println!("Description: {}", meta.description);
    if let Some(ref license) = meta.license {
        println!("License: {license}");
    }
    if let Some(ref compat) = meta.compatibility {
        println!("Compatibility: {compat}");
    }
    if let Some(version) = meta.version() {
        println!("Version: {version}");
    }
    if let Some(ref metadata) = meta.metadata {
        for (k, v) in metadata {
            if k != "version" {
                println!("  {k}: {v}");
            }
        }
    }
    Ok(())
}
