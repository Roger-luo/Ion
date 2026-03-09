use ion_skill::installer::data_dir;
use ion_skill::registry::Registry;

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

    let data = data_dir();

    if json {
        let entries: Vec<serde_json::Value> = removed
            .iter()
            .map(|(hash, url)| {
                let repo_dir = data.join(hash);
                serde_json::json!({
                    "hash": hash,
                    "url": url,
                    "directory": repo_dir.display().to_string(),
                    "exists": repo_dir.exists(),
                })
            })
            .collect();

        if !dry_run {
            for (hash, _url) in &removed {
                let repo_dir = data.join(hash);
                if repo_dir.exists() {
                    std::fs::remove_dir_all(&repo_dir)?;
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
        let repo_dir = data.join(hash);
        if dry_run {
            println!("Would remove: {url} ({hash})");
            if repo_dir.exists() {
                println!("  Directory: {}", repo_dir.display());
            }
        } else {
            println!("Removing: {url} ({hash})");
            if repo_dir.exists() {
                std::fs::remove_dir_all(&repo_dir)?;
                println!("  Deleted {}", repo_dir.display());
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
