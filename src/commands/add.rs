use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

use crate::context::ProjectContext;
use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let expanded = ctx.global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    let locked = match installer.install(&name, &source) {
        Ok(locked) => locked,
        Err(SkillError::ValidationWarning { report, .. }) => {
            print_validation_report(&name, &report);
            if !confirm_install_on_warnings()? {
                anyhow::bail!("Installation cancelled due to validation warnings.");
            }

            installer.install_with_options(
                &name,
                &source,
                InstallValidationOptions {
                    skip_validation: false,
                    allow_warnings: true,
                },
            )?
        }
        Err(err) => return Err(err.into()),
    };
    println!("  Installed to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    manifest_writer::add_skill(&ctx.manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}

fn skill_name_from_source(source: &SkillSource) -> String {
    if let Some(ref path) = source.path {
        path.rsplit('/').next().unwrap_or(path).to_string()
    } else {
        source
            .source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&source.source)
            .to_string()
    }
}
