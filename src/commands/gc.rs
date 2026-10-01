use ion_skill::installer::data_dir;
use ion_skill::registry::Registry;

pub fn list(json: bool) -> anyhow::Result<()> {
    let registry = Registry::load()?;

    if registry.repos.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!({ "repos": [] }));
        } else {
            println!("No cached skill repositories.");
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = registry
            .repos
            .iter()
            .map(|(hash, entry)| {
                let dirs = storage_dirs(hash);
                let repo_dir = &dirs[0];
                let size_bytes: u64 = dirs.iter().map(|path| dir_size(path)).sum();
                serde_json::json!({
                    "hash": hash,
                    "url": entry.url,
                    "directory": repo_dir.display().to_string(),
                    "directories": dirs.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                    "exists": repo_dir.exists(),
                    "size_bytes": size_bytes,
                    "projects": entry.projects,
                })
            })
            .collect();
        crate::json::print_success(serde_json::json!({ "repos": entries }));
        return Ok(());
    }

    println!("{} cached repo(s):", registry.repos.len());
    for (hash, entry) in &registry.repos {
        let dirs = storage_dirs(hash);
        let repo_dir = &dirs[0];
        let size_str = if repo_dir.exists() {
            format_size(dirs.iter().map(|path| dir_size(path)).sum())
        } else {
            "missing".to_string()
        };
        println!("  {} ({})", entry.url, size_str);
        for project in &entry.projects {
            println!("    used by {project}");
        }
    }

    Ok(())
}

/// Keep legacy deployments until the URL is unused by every registered project.
/// Prefer the current layout for the existing singular JSON directory field.
fn storage_dirs(hash: &str) -> Vec<std::path::PathBuf> {
    let data = data_dir();
    let modern = data.join("git").join(hash);
    let legacy = data.join(hash);
    if modern.exists() {
        vec![modern, legacy]
    } else {
        vec![legacy]
    }
}

fn dir_size(path: &std::path::Path) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| {
            let p = entry.path();
            if p.is_dir() {
                dir_size(&p)
            } else {
                p.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn run(dry_run: bool, json: bool) -> anyhow::Result<()> {
    let mut registry = Registry::load()?;

    let removed = registry.cleanup_stale();

    if removed.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!({
                "dry_run": dry_run,
                "removed": [],
            }));
            return Ok(());
        }
        println!("No stale repos to clean up.");
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = removed
            .iter()
            .map(|(hash, url)| {
                let dirs = storage_dirs(hash);
                let repo_dir = &dirs[0];
                serde_json::json!({
                    "hash": hash,
                    "url": url,
                    "directory": repo_dir.display().to_string(),
                    "directories": dirs.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                    "exists": repo_dir.exists(),
                })
            })
            .collect();

        if !dry_run {
            for (hash, _url) in &removed {
                let dirs = storage_dirs(hash);
                for repo_dir in dirs {
                    if repo_dir.exists() {
                        std::fs::remove_dir_all(&repo_dir)?;
                    }
                }
            }
            registry.save()?;
        }

        crate::json::print_success(serde_json::json!({
            "dry_run": dry_run,
            "removed": entries,
        }));
        return Ok(());
    }

    for (hash, url) in &removed {
        let dirs = storage_dirs(hash);
        if dry_run {
            println!("Would remove: {url} ({hash})");
            for repo_dir in &dirs {
                if repo_dir.exists() {
                    println!("  Directory: {}", repo_dir.display());
                }
            }
        } else {
            println!("Removing: {url} ({hash})");
            for repo_dir in &dirs {
                if repo_dir.exists() {
                    std::fs::remove_dir_all(repo_dir)?;
                    println!("  Deleted {}", repo_dir.display());
                }
            }
        }
    }

    if !dry_run {
        registry.save()?;
        println!("Cleaned up {} stale repo(s).", removed.len());
    } else {
        println!("{} repo(s) would be cleaned up.", removed.len());
    }

    Ok(())
}
