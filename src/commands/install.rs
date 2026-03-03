use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller, hash_simple};
use ion_skill::manifest::Manifest;
use ion_skill::registry::Registry;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;
use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};
use crate::style::Paint;

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in Ion.toml.");
        return Ok(());
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Installing {} skill(s)...", p.bold(&manifest.skills.len().to_string()));

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing {}...", p.bold(&format!("'{name}'")));
        let locked = match installer.install(name, &source) {
            Ok(locked) => locked,
            Err(SkillError::ValidationWarning { report, .. }) => {
                print_validation_report(name, &report);
                if !confirm_install_on_warnings()? {
                    anyhow::bail!(
                        "Installation of '{name}' cancelled due to validation warnings."
                    );
                }

                installer.install_with_options(
                    name,
                    &source,
                    InstallValidationOptions {
                        skip_validation: false,
                        allow_warnings: true,
                    },
                )?
            }
            Err(err) => return Err(err.into()),
        };

        // Add per-skill gitignore entries for remote skills only
        if source.source_type != SourceType::Path {
            let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
            ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
        }

        // Register in global registry for git-based sources
        if matches!(source.source_type, SourceType::Github | SourceType::Git)
            && let Ok(url) = source.git_url()
        {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = ctx.project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.register(&repo_hash, &url, &project_str);
            registry.save()?;
        }

        lockfile.upsert(locked);
    }

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("Updated {}", p.dim("Ion.lock"));
    println!("{}", p.success("Done!"));

    Ok(())
}
