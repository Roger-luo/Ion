use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller, hash_simple};
use ion_skill::lockfile::LockedSkill;
use ion_skill::manifest::ManifestOptions;
use ion_skill::registry::Registry;
use ion_skill::source::{SkillSource, SourceType};
use ion_skill::validate::ValidationReport;

use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

/// A skill ready for installation (validated, source resolved).
pub struct SkillEntry {
    pub name: String,
    pub source: SkillSource,
}

/// Register a git-based skill source in the global registry.
pub fn register_in_registry(
    source: &SkillSource,
    project_dir: &std::path::Path,
) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git)
        && let Ok(url) = source.git_url()
    {
        let repo_hash = format!("{:x}", hash_simple(&url));
        let project_str = project_dir.display().to_string();
        let mut registry = Registry::load()?;
        registry.register(&repo_hash, &url, &project_str);
        registry.save()?;
    }
    Ok(())
}

/// Install a skill, handling validation warnings interactively.
pub fn install_with_warning_prompt(
    installer: &SkillInstaller,
    name: &str,
    source: &SkillSource,
    json: bool,
    allow_warnings: bool,
) -> anyhow::Result<LockedSkill> {
    match installer.install(name, source) {
        Ok(locked) => Ok(locked),
        Err(SkillError::ValidationWarning { report, .. }) => {
            handle_validation_warnings(name, &report, json, allow_warnings)?;
            let locked = installer.install_with_options(
                name,
                source,
                InstallValidationOptions {
                    skip_validation: false,
                    allow_warnings: true,
                },
            )?;
            Ok(locked)
        }
        Err(err) => Err(err.into()),
    }
}

/// Display validation warnings and prompt for confirmation (or exit in JSON mode).
pub fn handle_validation_warnings(
    name: &str,
    report: &ValidationReport,
    json: bool,
    allow_warnings: bool,
) -> anyhow::Result<()> {
    if json && !allow_warnings {
        crate::json::print_action_required(
            "validation_warnings",
            serde_json::json!({
                "skill": name,
                "warnings": &report.findings,
            }),
        );
        // print_action_required calls process::exit, so this is unreachable
    }
    if !json {
        print_validation_report(name, report);
        if !confirm_install_on_warnings()? {
            anyhow::bail!("Installation cancelled due to validation warnings.");
        }
    }
    Ok(())
}

/// Add gitignore entries for a remote skill (skips Path/Local sources).
pub fn add_gitignore_entries(
    project_dir: &std::path::Path,
    name: &str,
    source: &SkillSource,
    merged_options: &ManifestOptions,
) -> anyhow::Result<()> {
    if !matches!(source.source_type, SourceType::Path | SourceType::Local) {
        let target_paths: Vec<&str> = merged_options
            .targets
            .values()
            .map(|s| s.as_str())
            .collect();
        ion_skill::gitignore::add_skill_entries(project_dir, name, &target_paths)?;
    }
    Ok(())
}
