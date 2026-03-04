use std::path::PathBuf;

use ion_skill::installer::SkillInstaller;
use ion_skill::manifest_writer;
use ion_skill::source::{SkillSource, SourceType};

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(path: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);

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

    println!("Linking local skill {} from {}...", p.bold(&format!("'{name}'")), p.info(path));

    // Build a path source pointing to the local directory
    let source = SkillSource {
        source_type: SourceType::Path,
        source: path.to_string(),
        path: None,
        rev: None,
        version: None,
        binary: None,
    };

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    let locked = installer.install(&name, &source)?;

    println!("  Linked to {}", p.info(&format!(".agents/skills/{name}/")));
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {}", p.info(target_name));
    }

    // No gitignore entries for local skills — they should be tracked in git

    manifest_writer::add_skill(&ctx.manifest_path, &name, &source)?;
    println!("  Updated {}", p.dim("Ion.toml"));

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated {}", p.dim("Ion.lock"));

    println!("{}", p.success("Done!"));
    crate::commands::init::print_no_targets_hint(&merged_options, &p);
    Ok(())
}
