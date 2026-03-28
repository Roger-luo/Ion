use std::path::PathBuf;

use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

use crate::context::ProjectContext;

pub fn run(path: &str, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = ctx.paint();

    let skill_path = if PathBuf::from(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    if !skill_path.exists() {
        anyhow::bail!("Path does not exist: {}", skill_path.display());
    }

    let skill_md = skill_path.join("SKILL.md");
    if !skill_md.exists() {
        anyhow::bail!(
            "No SKILL.md found at {}. Is this a skill directory?",
            skill_path.display()
        );
    }

    // Read name from SKILL.md metadata
    let (meta, _body) = ion_skill::skill::SkillMetadata::from_file(&skill_md)?;
    let name = meta.name.clone();

    if !json {
        println!(
            "Linking local skill {} from {}...",
            p.bold(&format!("'{name}'")),
            p.info(path)
        );
    }

    // Build a path source pointing to the local directory
    let source = SkillSource::from_path(path);

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let installer = ctx.installer(&merged_options);
    let locked = installer.install(&name, &source)?;

    if !json {
        println!(
            "  Linked to {}",
            p.info(&format!(
                "{}/{name}/",
                merged_options.skills_dir_or_default()
            ))
        );
        for target_name in merged_options.targets.keys() {
            println!("  Linked to {}", p.info(target_name));
        }
    }

    // No gitignore entries for local skills — they should be tracked in git

    manifest_writer::add_skill(&ctx.manifest_path, &name, &source)?;
    if !json {
        println!("  Updated {}", p.dim("Ion.toml"));
    }

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    if !json {
        println!("  Updated {}", p.dim("Ion.lock"));
    }

    if json {
        let targets: Vec<&str> = merged_options.targets.keys().map(|s| s.as_str()).collect();
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": path,
            "targets": targets,
        }));
        return Ok(());
    }

    println!("{}", p.success("Done!"));
    crate::commands::init::print_no_targets_hint(&merged_options, &p, json);
    Ok(())
}
