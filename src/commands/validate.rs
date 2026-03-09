use std::path::{Path, PathBuf};

use ion_skill::skill::SkillMetadata;
use ion_skill::validate::discovery::discover_skill_files;
use ion_skill::validate::validate_skill_dir;

pub fn run(path: Option<&str>, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let target = match path {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.is_absolute() { p } else { cwd.join(p) }
        }
        None => cwd.clone(),
    };

    let skill_files = resolve_skill_files(path.is_some(), &target)?;
    if skill_files.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!({
                "skills": [],
                "total_errors": 0,
                "total_warnings": 0,
                "total_infos": 0,
            }));
            return Ok(());
        }
        println!("No SKILL.md files found under {}", target.display());
        return Ok(());
    }

    if !json {
        println!("Validating {} skill(s)...", skill_files.len());
    }

    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut total_infos = 0usize;
    let mut json_skills: Vec<serde_json::Value> = Vec::new();

    for skill_md in &skill_files {
        if !json {
            println!("\n{}", skill_md.display());
        }
        let skill_dir = skill_md
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid skill path: {}", skill_md.display()))?;

        match SkillMetadata::from_file(skill_md) {
            Ok((meta, body)) => {
                let report = validate_skill_dir(skill_dir, &meta, &body);
                if json {
                    let findings: Vec<serde_json::Value> = report
                        .findings
                        .iter()
                        .map(|f| {
                            serde_json::json!({
                                "severity": f.severity.to_string(),
                                "checker": f.checker,
                                "message": f.message,
                                "detail": f.detail,
                            })
                        })
                        .collect();
                    json_skills.push(serde_json::json!({
                        "path": skill_md.display().to_string(),
                        "name": meta.name,
                        "findings": findings,
                        "errors": report.error_count,
                        "warnings": report.warning_count,
                        "infos": report.info_count,
                    }));
                } else if report.findings.is_empty() {
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
                if json {
                    json_skills.push(serde_json::json!({
                        "path": skill_md.display().to_string(),
                        "findings": [{"severity": "error", "checker": "schema", "message": err.to_string(), "detail": null}],
                        "errors": 1,
                        "warnings": 0,
                        "infos": 0,
                    }));
                } else {
                    println!("  ERROR [schema] {err}");
                }
                total_errors += 1;
            }
        }
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "skills": json_skills,
            "total_errors": total_errors,
            "total_warnings": total_warnings,
            "total_infos": total_infos,
        }));
        if total_errors > 0 {
            std::process::exit(1);
        }
        return Ok(());
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
