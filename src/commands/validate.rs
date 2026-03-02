use std::path::{Path, PathBuf};

use ion_skill::skill::SkillMetadata;
use ion_skill::validate::discovery::discover_skill_files;
use ion_skill::validate::validate_skill_dir;

pub fn run(path: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let target = match path {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                cwd.join(p)
            }
        }
        None => cwd.clone(),
    };

    let skill_files = resolve_skill_files(path.is_some(), &target)?;
    if skill_files.is_empty() {
        println!("No SKILL.md files found under {}", target.display());
        return Ok(());
    }

    println!("Validating {} skill(s)...", skill_files.len());

    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut total_infos = 0usize;

    for skill_md in skill_files {
        println!("\n{}", skill_md.display());
        let skill_dir = skill_md
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid skill path: {}", skill_md.display()))?;

        match SkillMetadata::from_file(&skill_md) {
            Ok((meta, body)) => {
                let report = validate_skill_dir(skill_dir, &meta, &body);
                if report.findings.is_empty() {
                    println!("  OK (no findings)");
                } else {
                    for finding in &report.findings {
                        println!(
                            "  {} [{}] {}",
                            finding.severity, finding.checker, finding.message
                        );
                        if let Some(detail) = &finding.detail {
                            println!("    {detail}");
                        }
                    }
                }
                total_errors += report.error_count;
                total_warnings += report.warning_count;
                total_infos += report.info_count;
            }
            Err(err) => {
                println!("  ERROR [schema] {err}");
                total_errors += 1;
            }
        }
    }

    println!(
        "\nSummary: {} error(s), {} warning(s), {} info",
        total_errors, total_warnings, total_infos
    );

    if total_errors > 0 {
        anyhow::bail!("Validation failed with {total_errors} error(s).");
    }

    Ok(())
}

fn resolve_skill_files(explicit_path: bool, target: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if target.is_file() {
        if target.file_name().is_some_and(|name| name == "SKILL.md") {
            return Ok(vec![target.to_path_buf()]);
        }
        anyhow::bail!("Expected a SKILL.md file, got {}", target.display());
    }

    if target.is_dir() {
        let direct_skill = target.join("SKILL.md");
        if explicit_path && direct_skill.exists() {
            return Ok(vec![direct_skill]);
        }

        let mut files = discover_skill_files(target)?;
        files.sort();
        files.dedup();
        return Ok(files);
    }

    anyhow::bail!("Path does not exist: {}", target.display())
}
