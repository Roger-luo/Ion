use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let merged_options = ctx.merged_options(&manifest);
    let lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!([]));
            return Ok(());
        }
        println!("No skills declared in Ion.toml.");
        return Ok(());
    }

    if json {
        let skills: Vec<serde_json::Value> = manifest
            .skills
            .iter()
            .filter_map(|(name, entry)| {
                let source = match entry.resolve() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Warning: skipping '{}': {}", name, e);
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
                let installed = ctx
                    .project_dir
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
        crate::json::print_success(serde_json::json!(skills));
        return Ok(());
    }

    println!("Skills ({}):", p.bold(&manifest.skills.len().to_string()));
    for (name, entry) in &manifest.skills {
        let source = match entry.resolve() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: skipping '{}': {}", name, e);
                continue;
            }
        };
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

        let installed = ctx
            .project_dir
            .join(merged_options.skills_dir_or_default())
            .join(name)
            .exists();
        let status = if installed {
            p.success("installed")
        } else {
            p.warn("not installed")
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
    Ok(())
}
