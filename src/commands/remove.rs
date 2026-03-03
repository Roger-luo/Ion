use ion_skill::installer::{SkillInstaller, hash_simple};
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::registry::Registry;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;

pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let manifest = ctx.manifest()?;

    // If the argument matches a skill name directly, remove that single skill.
    // Otherwise, treat it as a source prefix and remove all matching skills.
    let skills_to_remove: Vec<String> = if manifest.skills.contains_key(name) {
        vec![name.to_string()]
    } else {
        let matches: Vec<String> = manifest
            .skills
            .iter()
            .filter(|(_, entry)| source_matches(entry, name))
            .map(|(skill_name, _)| skill_name.clone())
            .collect();
        if matches.is_empty() {
            anyhow::bail!("No skills matching '{name}' found in Ion.toml");
        }
        matches
    };

    let merged_options = ctx.merged_options(&manifest);
    let mut lockfile = ctx.lockfile()?;

    for skill_name in &skills_to_remove {
        let entry = &manifest.skills[skill_name];

        println!("Removing skill '{skill_name}'...");

        SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(skill_name)?;
        println!("  Removed from .agents/skills/{skill_name}/");

        ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, skill_name)?;
        println!("  Updated .gitignore");

        // Unregister from global registry for git-based sources
        if let Ok(source) = Manifest::resolve_entry(entry)
            && matches!(source.source_type, SourceType::Github | SourceType::Git)
            && let Ok(url) = source.git_url()
        {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = ctx.project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.unregister(&repo_hash, &project_str);
            registry.save()?;
        }

        manifest_writer::remove_skill(&ctx.manifest_path, skill_name)?;
        lockfile.remove(skill_name);
    }

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated Ion.toml");
    println!("  Updated Ion.lock");
    println!("Done!");
    Ok(())
}

/// Check if a skill entry's source matches or starts with the given query.
/// Supports matching by repo (e.g. "obra/superpowers") or full path
/// (e.g. "obra/superpowers/skills/brainstorming").
fn source_matches(entry: &ion_skill::manifest::SkillEntry, query: &str) -> bool {
    let source_str = match entry {
        ion_skill::manifest::SkillEntry::Shorthand(s) => s.as_str(),
        ion_skill::manifest::SkillEntry::Full { source, .. } => source.as_str(),
    };
    // Match if the entry source string equals or starts with the query
    // e.g. query "obra/superpowers" matches "obra/superpowers/skills/brainstorming"
    source_str == query || source_str.starts_with(&format!("{query}/"))
}
