use crate::context::WorkspaceContext;

/// Add a sub-project to the workspace.
pub fn add(path: &str, json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let root = ws.root_project();
    let p = ws.paint();

    // Create member directory if missing
    let member_dir = root.dir.join(path);
    if !member_dir.exists() {
        std::fs::create_dir_all(&member_dir)?;
    }

    // Create member Ion.toml if missing
    let member_manifest = member_dir.join("Ion.toml");
    if !member_manifest.exists() {
        std::fs::write(&member_manifest, "[skills]\n")?;
    }

    // Add to workspace members list
    ion_skill::manifest_writer::add_workspace_member(&root.manifest_path, path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "added": path,
        }));
    } else {
        println!("{} workspace member '{}'", p.success("Added"), p.bold(path));
    }

    Ok(())
}

/// Remove a sub-project from the workspace.
pub fn remove(path: &str, json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let root = ws.root_project();
    let p = ws.paint();

    ion_skill::manifest_writer::remove_workspace_member(&root.manifest_path, path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "removed": path,
        }));
    } else {
        println!(
            "{} workspace member '{}' (files preserved)",
            p.success("Removed"),
            p.bold(path)
        );
    }

    Ok(())
}

/// List all workspace members.
pub fn list(json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();

    if json {
        let members: Vec<serde_json::Value> = ws
            .projects
            .iter()
            .enumerate()
            .map(|(i, project)| {
                let label = if i == 0 {
                    ". (root)".to_string()
                } else {
                    project
                        .dir
                        .strip_prefix(ws.root_dir())
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| project.dir.display().to_string())
                };
                let skill_count = project
                    .manifest_or_empty()
                    .map(|m| m.skills.len())
                    .unwrap_or(0);
                serde_json::json!({
                    "path": label,
                    "skill_count": skill_count,
                })
            })
            .collect();
        crate::json::print_success(serde_json::json!(members));
        return Ok(());
    }

    if !ws.is_workspace() {
        println!("Not a workspace (no [workspace] section in Ion.toml).");
        return Ok(());
    }

    println!("Workspace members:");
    for (i, project) in ws.projects.iter().enumerate() {
        let label = if i == 0 {
            ". (root)".to_string()
        } else {
            project
                .dir
                .strip_prefix(ws.root_dir())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| project.dir.display().to_string())
        };
        let skill_count = project
            .manifest_or_empty()
            .map(|m| m.skills.len())
            .unwrap_or(0);
        println!("  {:<30} {} skill(s)", p.bold(&label), skill_count);
    }

    Ok(())
}

/// Show workspace status.
pub fn status(json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();

    if json {
        let members: Vec<serde_json::Value> = ws
            .projects
            .iter()
            .enumerate()
            .map(|(i, project)| {
                let label = if i == 0 {
                    ". (root)".to_string()
                } else {
                    project
                        .dir
                        .strip_prefix(ws.root_dir())
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| project.dir.display().to_string())
                };
                let skill_count = project
                    .manifest_or_empty()
                    .map(|m| m.skills.len())
                    .unwrap_or(0);
                let has_manifest = project.has_manifest();
                serde_json::json!({
                    "path": label,
                    "skill_count": skill_count,
                    "has_manifest": has_manifest,
                })
            })
            .collect();
        crate::json::print_success(serde_json::json!({
            "is_workspace": ws.is_workspace(),
            "members": members,
        }));
        return Ok(());
    }

    if !ws.is_workspace() {
        println!("Not a workspace (no [workspace] section in Ion.toml).");
        return Ok(());
    }

    println!("Workspace status:");
    for (i, project) in ws.projects.iter().enumerate() {
        let label = if i == 0 {
            ". (root)".to_string()
        } else {
            project
                .dir
                .strip_prefix(ws.root_dir())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| project.dir.display().to_string())
        };
        let skill_count = project
            .manifest_or_empty()
            .map(|m| m.skills.len())
            .unwrap_or(0);
        let has_manifest = project.has_manifest();
        let status_str = if has_manifest {
            p.success("ok")
        } else {
            p.warn("no Ion.toml")
        };
        println!(
            "  {:<30} {} skill(s)  [{}]",
            p.bold(&label),
            skill_count,
            status_str
        );
    }

    Ok(())
}
