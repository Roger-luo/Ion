use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::manifest::Manifest;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;
use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Installing {} skill(s)...", manifest.skills.len());

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing '{name}'...");
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

        lockfile.upsert(locked);
    }

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");

    Ok(())
}
