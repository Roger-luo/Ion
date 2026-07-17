use crate::context::WorkspaceContext;
use ion_skill::workspace::Project;

pub fn run(json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let projects = ws.scoped_projects();
    let p = ws.paint();
    let multi = projects.len() > 1;

    if json {
        let mut all_skills: Vec<serde_json::Value> = Vec::new();
        for project in &projects {
            if !project.has_manifest() {
                continue;
            }
            let project_label = project_label(project, &ws);
            let skills = json_skills_for_project(project, &ws)?;
            for mut skill in skills {
                skill
                    .as_object_mut()
                    .unwrap()
                    .insert("project".to_string(), serde_json::json!(project_label));
                all_skills.push(skill);
            }
        }
        crate::json::print_success(serde_json::json!(all_skills));
        return Ok(());
    }

    let mut any_skills = false;
    for project in &projects {
        if !project.has_manifest() {
            continue;
        }
        let manifest = project.manifest()?;
        if manifest.skills.is_empty() {
            continue;
        }
        any_skills = true;

        if multi {
            let label = project_label(project, &ws);
            println!("\n{}:", p.bold(&label));
        }

        let merged_options = ws.merged_options_for(project)?;
        let lockfile = project.lockfile()?;

        for (name, entry) in &manifest.skills {
            let source = match entry.resolve() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: skipping '{}': {}", name, e);
                    continue;
                }
            };
            let installed = project
                .dir
                .join(merged_options.skills_dir_or_default())
                .join(name)
                .exists();
            let status = if installed {
                p.success("installed")
            } else {
                p.warn("not installed")
            };

            // Local skills have no version/commit to report (they're managed
            // directly by git, not by ion's fetch pipeline) and an empty
            // `source.source` — printing them the same way as remote skills
            // produced a misleading "vunknown" version and a blank source line.
            if source.is_local() {
                println!("  {} {} [{}]", p.bold(name), p.dim("(local)"), status);
                println!("    source: {}", p.info("local"));
                continue;
            }

            let locked = lockfile.find(name);

            let is_binary = locked.is_some_and(|l| l.is_binary());

            let version_str = if is_binary {
                locked.and_then(|l| l.binary_version()).unwrap_or("unknown")
            } else {
                locked
                    .and_then(|l| l.version.as_deref())
                    .unwrap_or("unknown")
            };

            let type_indicator = if is_binary {
                format!(" {}", p.info("(binary)"))
            } else {
                let commit_str = locked
                    .and_then(|l| l.commit())
                    .map(|c| &c[..c.len().min(8)])
                    .unwrap_or("none");
                format!(" {}", p.dim(&format!("({commit_str})")))
            };

            let display_version = if version_str.starts_with('v') {
                version_str.to_string()
            } else {
                format!("v{version_str}")
            };
            println!(
                "  {} {}{} [{}]",
                p.bold(name),
                p.dim(&display_version),
                type_indicator,
                status
            );
            println!("    source: {}", p.info(&source.source));
        }
    }

    if !any_skills {
        println!("No skills declared in Ion.toml.");
    }

    Ok(())
}

/// Build JSON skill entries for a single project.
fn json_skills_for_project(
    project: &Project,
    ws: &WorkspaceContext,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let manifest = project.manifest()?;
    let merged_options = ws.merged_options_for(project)?;
    let lockfile = project.lockfile()?;

    let skills: Vec<serde_json::Value> = manifest
        .skills
        .iter()
        .filter_map(|(name, entry)| {
            let source = match entry.resolve() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: skipping '{}': {}", name, e);
                    return None;
                }
            };
            let locked = lockfile.find(name);
            let is_binary = locked.is_some_and(|l| l.is_binary());
            let version = if is_binary {
                locked.and_then(|l| l.binary_version()).unwrap_or("unknown")
            } else {
                locked
                    .and_then(|l| l.version.as_deref())
                    .unwrap_or("unknown")
            };
            let commit = locked.and_then(|l| l.commit());
            let installed = project
                .dir
                .join(merged_options.skills_dir_or_default())
                .join(name)
                .exists();
            Some(serde_json::json!({
                "name": name,
                "source": source.source,
                "version": version,
                "commit": commit,
                "binary": is_binary,
                "installed": installed,
            }))
        })
        .collect();
    Ok(skills)
}

/// Human-readable label for a project within a workspace.
fn project_label(project: &Project, ws: &WorkspaceContext) -> String {
    let root_dir = ws.root_dir();
    if project.dir == root_dir {
        ". (root)".to_string()
    } else {
        project
            .dir
            .strip_prefix(root_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| project.dir.display().to_string())
    }
}
