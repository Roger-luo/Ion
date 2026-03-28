use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller, hash_simple};
use ion_skill::lockfile::{LockedSkill, Lockfile};
use ion_skill::manifest::ManifestOptions;
use ion_skill::manifest_writer;
use ion_skill::registry::Registry;
use ion_skill::source::SkillSource;
use ion_skill::validate::ValidationReport;

use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};
use crate::context::ProjectContext;
use crate::style::Paint;

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
    if source.is_git_based()
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

/// Unregister a git-based skill source from the global registry.
pub fn unregister_from_registry(
    source: &SkillSource,
    project_dir: &std::path::Path,
) -> anyhow::Result<()> {
    if source.is_git_based()
        && let Ok(url) = source.git_url()
    {
        let repo_hash = format!("{:x}", hash_simple(&url));
        let project_str = project_dir.display().to_string();
        let mut registry = Registry::load()?;
        registry.unregister(&repo_hash, &project_str);
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
    if source.is_remote_installable() {
        let target_paths: Vec<&str> = merged_options
            .targets
            .values()
            .map(|s| s.as_str())
            .collect();
        ion_skill::gitignore::add_skill_entries(
            project_dir,
            name,
            &target_paths,
            merged_options.skills_dir_or_default(),
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared finalize helpers
// ---------------------------------------------------------------------------

/// Options controlling which post-install steps to perform.
pub struct FinalizeOptions {
    /// Write a new entry to Ion.toml (false for `ion install` since skills are already declared).
    pub write_manifest: bool,
    /// Register in global registry (false when caller handles registry once at batch end).
    pub register_in_registry: bool,
}

impl FinalizeOptions {
    /// For `ion add` (single skill): write manifest, register.
    pub const ADD: Self = Self {
        write_manifest: true,
        register_in_registry: true,
    };
    /// For `ion install` (skills already in Ion.toml): no manifest write, register.
    pub const INSTALL: Self = Self {
        write_manifest: false,
        register_in_registry: true,
    };
    /// For `ion add` collection loop: write manifest per-skill, registry once at end.
    pub const ADD_COLLECTION: Self = Self {
        write_manifest: true,
        register_in_registry: false,
    };
}

/// Post-install bookkeeping: conditionally does gitignore, registry, manifest, lockfile.
pub fn finalize_skill_install(
    ctx: &ProjectContext,
    merged_options: &ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: LockedSkill,
    lockfile: &mut Lockfile,
    opts: &FinalizeOptions,
) -> anyhow::Result<()> {
    add_gitignore_entries(&ctx.project_dir, name, source, merged_options)?;
    if opts.register_in_registry {
        register_in_registry(source, &ctx.project_dir)?;
    }
    if opts.write_manifest {
        manifest_writer::add_skill(&ctx.manifest_path, name, source)?;
    }
    lockfile.upsert(locked);
    Ok(())
}

/// Post-install bookkeeping + write lockfile (for single-skill commands like add/link).
pub fn finalize_skill_install_and_write(
    ctx: &ProjectContext,
    merged_options: &ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: LockedSkill,
    opts: &FinalizeOptions,
) -> anyhow::Result<()> {
    let mut lockfile = ctx.lockfile()?;
    finalize_skill_install(
        ctx,
        merged_options,
        name,
        source,
        locked,
        &mut lockfile,
        opts,
    )?;
    lockfile.write_to(&ctx.lockfile_path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Validation buckets & batch install
// ---------------------------------------------------------------------------

/// Results of validating a batch of skills (excludes local skills — handle those separately).
pub struct ValidationBuckets {
    pub clean: Vec<SkillEntry>,
    pub warned: Vec<(SkillEntry, ValidationReport)>,
    pub errored: Vec<(String, ValidationReport)>,
}

impl ValidationBuckets {
    /// Validate a set of (name, source) pairs, bucketing by result.
    /// Callers MUST filter out Local skills before calling this.
    pub fn collect(
        installer: &SkillInstaller,
        skills: impl IntoIterator<Item = (String, SkillSource)>,
    ) -> anyhow::Result<Self> {
        let mut clean = Vec::new();
        let mut warned = Vec::new();
        let mut errored = Vec::new();

        for (name, source) in skills {
            match installer.validate(&name, &source) {
                Ok(report) if report.warning_count > 0 => {
                    warned.push((SkillEntry { name, source }, report));
                }
                Ok(_) => {
                    clean.push(SkillEntry { name, source });
                }
                Err(SkillError::ValidationFailed { report, .. }) => {
                    errored.push((name, report));
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(Self {
            clean,
            warned,
            errored,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.clean.is_empty() && self.warned.is_empty() && self.errored.is_empty()
    }
}

/// Install approved skills from validation buckets.
/// The `finalize` callback controls post-install bookkeeping.
pub fn install_approved_skills(
    installer: &SkillInstaller,
    buckets: &ValidationBuckets,
    warned_selections: &[bool],
    p: &Paint,
    json: bool,
    mut finalize: impl FnMut(&str, &SkillSource, LockedSkill) -> anyhow::Result<()>,
) -> anyhow::Result<usize> {
    let mut installed = 0;

    for entry in &buckets.clean {
        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions::default(),
        )?;
        finalize(&entry.name, &entry.source, locked)?;
        installed += 1;
    }

    for (i, (entry, _)) in buckets.warned.iter().enumerate() {
        if !warned_selections.get(i).copied().unwrap_or(false) {
            if !json {
                println!("  Skipping '{}' (deselected)", entry.name);
            }
            continue;
        }
        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name,
            &entry.source,
            InstallValidationOptions {
                skip_validation: false,
                allow_warnings: true,
            },
        )?;
        finalize(&entry.name, &entry.source, locked)?;
        installed += 1;
    }

    Ok(installed)
}
