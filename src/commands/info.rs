use ion_skill::binary;
use ion_skill::skill::SkillMetadata;
use ion_skill::source::SkillSource;

use crate::context::ProjectContext;

pub fn run(skill_str: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    if ctx.manifest_path.exists() {
        let manifest = ctx.manifest()?;
        if manifest.skills.contains_key(skill_str) {
            return show_info_from_installed(&ctx, skill_str);
        }
    }

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

fn show_info_from_installed(ctx: &ProjectContext, name: &str) -> anyhow::Result<()> {
    let skill_md = ctx
        .project_dir
        .join(".agents")
        .join("skills")
        .join(name)
        .join("SKILL.md");

    if !skill_md.exists() {
        anyhow::bail!("Skill '{name}' is in Ion.toml but not installed. Run `ion install`.");
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

    // Binary-specific info
    let lockfile = ctx.lockfile()?;
    if let Some(locked) = lockfile.find(name) {
        if let Some(ref binary_name) = locked.binary {
            println!("Binary: {binary_name}");
            if let Some(ref binary_version) = locked.binary_version {
                println!("Binary version: {binary_version}");
                let bin_path = binary::binary_path(binary_name, binary_version);
                println!("Binary path: {}", bin_path.display());
                if bin_path.exists() {
                    if let Ok(metadata) = std::fs::metadata(&bin_path) {
                        println!("Binary size: {}", format_size(metadata.len()));
                    }
                }
            }
            println!("Run with: ion run {} [args]", name);
        }
    }

    // Other metadata
    if let Some(ref metadata) = meta.metadata {
        for (k, v) in metadata {
            if k != "version" && k != "binary" {
                println!("  {k}: {v}");
            }
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
