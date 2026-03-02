use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::manifest::Manifest;

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
        lockfile.upsert(locked);
    }

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");

    // Check gitignore for managed directories
    let mut managed_dirs = vec![".agents/".to_string()];
    for path in merged_options.targets.values() {
        let top_level = path.split('/').next().unwrap_or(path);
        let entry = format!("{top_level}/");
        if !managed_dirs.contains(&entry) {
            managed_dirs.push(entry);
        }
    }

    let dir_refs: Vec<&str> = managed_dirs.iter().map(|s| s.as_str()).collect();
    let missing =
        ion_skill::gitignore::find_missing_gitignore_entries(&ctx.project_dir, &dir_refs)?;

    if !missing.is_empty() {
        println!("\nThese directories are not in .gitignore:");
        for dir in &missing {
            println!("  {dir}");
        }
        print!("\nAdd them? [Y/n] (press Enter for yes) ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;

        if answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y") {
            let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
            ion_skill::gitignore::append_to_gitignore(&ctx.project_dir, &refs)?;
            println!("Updated .gitignore");
        }
    }

    Ok(())
}
