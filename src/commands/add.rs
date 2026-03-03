use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller, hash_simple};
use ion_skill::manifest_writer;
use ion_skill::registry::Registry;
use ion_skill::source::{SkillSource, SourceType};

use crate::context::ProjectContext;
use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let expanded = ctx.global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    // If the source has no path (i.e. points to a whole repo), check if it's
    // a multi-skill collection. Try to install as a single skill first; if there
    // is no root SKILL.md, discover and install all skills in the repo.
    if source.path.is_none() {
        let name = skill_name_from_source(&source);
        println!("Adding skill '{name}' from {source_str}...");

        let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
        match installer.install(&name, &source) {
            Ok(locked) => {
                return finish_single_install(&ctx, &installer, &merged_options, &name, &source, locked);
            }
            Err(SkillError::ValidationWarning { report, .. }) => {
                print_validation_report(&name, &report);
                if !confirm_install_on_warnings()? {
                    anyhow::bail!("Installation cancelled due to validation warnings.");
                }
                let locked = installer.install_with_options(
                    &name,
                    &source,
                    InstallValidationOptions {
                        skip_validation: false,
                        allow_warnings: true,
                    },
                )?;
                return finish_single_install(&ctx, &installer, &merged_options, &name, &source, locked);
            }
            Err(SkillError::InvalidSkill(msg)) if msg.contains("No SKILL.md found") => {
                // Not a single-skill repo — try as a multi-skill collection
                return install_collection(&ctx, &merged_options, &source, source_str);
            }
            Err(err) => return Err(err.into()),
        }
    }

    // Source has a path — install a single skill directly
    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

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

    finish_single_install(&ctx, &installer, &merged_options, &name, &source, locked)
}

fn install_collection(
    ctx: &ProjectContext,
    merged_options: &ion_skill::manifest::ManifestOptions,
    base_source: &SkillSource,
    source_str: &str,
) -> anyhow::Result<()> {
    let skills = SkillInstaller::discover_skills(base_source)?;
    if skills.is_empty() {
        anyhow::bail!("No skills found in repository '{source_str}'");
    }

    println!(
        "Found {} skill(s) in collection '{source_str}':",
        skills.len()
    );
    for (name, path) in &skills {
        println!("  {name} ({path})");
    }
    println!();

    let installer = SkillInstaller::new(&ctx.project_dir, merged_options);
    let mut lockfile = ctx.lockfile()?;

    for (name, path) in &skills {
        let mut skill_source = base_source.clone();
        skill_source.path = Some(path.clone());

        println!("  Installing '{name}'...");
        let locked = match installer.install(name, &skill_source) {
            Ok(locked) => locked,
            Err(SkillError::ValidationWarning { report, .. }) => {
                print_validation_report(name, &report);
                if !confirm_install_on_warnings()? {
                    println!("  Skipping '{name}' due to validation warnings.");
                    continue;
                }
                installer.install_with_options(
                    name,
                    &skill_source,
                    InstallValidationOptions {
                        skip_validation: false,
                        allow_warnings: true,
                    },
                )?
            }
            Err(SkillError::ValidationFailed { report, .. }) => {
                print_validation_report(name, &report);
                println!("  Skipping '{name}' due to validation errors.");
                continue;
            }
            Err(err) => return Err(err.into()),
        };

        println!("    Installed to .agents/skills/{name}/");
        for target_name in merged_options.targets.keys() {
            println!("    Linked to {target_name}");
        }

        // Add per-skill gitignore entries for remote skills
        if skill_source.source_type != SourceType::Path {
            let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
            ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
        }

        manifest_writer::add_skill(&ctx.manifest_path, name, &skill_source)?;
        lockfile.upsert(locked);
    }

    // Register in global registry (once for the base source)
    register_in_registry(base_source, &ctx.project_dir)?;

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated Ion.toml");
    println!("  Updated Ion.lock");
    println!("Done!");
    crate::commands::init::print_no_targets_hint(merged_options);
    Ok(())
}

fn finish_single_install(
    ctx: &ProjectContext,
    _installer: &SkillInstaller,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
) -> anyhow::Result<()> {
    println!("  Installed to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    // Add per-skill gitignore entries for remote skills only
    if source.source_type != SourceType::Path {
        let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
        ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
        println!("  Updated .gitignore");
    }

    // Register in global registry for git-based sources
    register_in_registry(source, &ctx.project_dir)?;

    manifest_writer::add_skill(&ctx.manifest_path, name, source)?;
    println!("  Updated Ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated Ion.lock");

    println!("Done!");
    crate::commands::init::print_no_targets_hint(merged_options);
    Ok(())
}

fn register_in_registry(source: &SkillSource, project_dir: &std::path::Path) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git) {
        if let Ok(url) = source.git_url() {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.register(&repo_hash, &url, &project_str);
            registry.save()?;
        }
    }
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
