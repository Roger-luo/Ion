use ion_skill::binary;
use ion_skill::manifest::Manifest;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    // Collect binary skills to update
    let skills_to_check: Vec<(String, _)> = manifest
        .skills
        .iter()
        .filter(|(skill_name, _)| {
            // If a name filter was given, only check that skill
            name.is_none() || name == Some(skill_name.as_str())
        })
        .filter_map(|(skill_name, entry)| {
            let source = Manifest::resolve_entry(entry).ok()?;
            if source.source_type == SourceType::Binary {
                Some((skill_name.clone(), source))
            } else {
                None
            }
        })
        .collect();

    if skills_to_check.is_empty() {
        if let Some(n) = name {
            anyhow::bail!("No binary skill '{}' found in Ion.toml", n);
        }
        println!("No binary skills to update.");
        return Ok(());
    }

    println!(
        "Checking {} binary skill(s) for updates...",
        p.bold(&skills_to_check.len().to_string())
    );

    let mut updated_count = 0;

    for (skill_name, source) in &skills_to_check {
        let binary_name = source.binary.as_deref().unwrap_or(skill_name.as_str());

        // Get current version from lockfile
        let current_version = lockfile
            .find(skill_name)
            .and_then(|l| l.binary_version.as_deref())
            .unwrap_or("unknown")
            .to_string();

        // Query GitHub for latest release
        print!("  {} ", p.bold(skill_name));
        let release = match binary::fetch_github_release(&source.source, source.rev.as_deref()) {
            Ok(r) => r,
            Err(e) => {
                println!("{}", p.warn(&format!("failed to check: {}", e)));
                continue;
            }
        };

        let latest_version = binary::parse_version_from_tag(&release.tag_name).to_string();

        if current_version == latest_version {
            println!("v{} {}", current_version, p.dim("(up to date)"));
            continue;
        }

        println!(
            "v{} → v{}",
            current_version,
            p.info(&latest_version)
        );

        // Install the new version
        let skill_dir = ctx
            .project_dir
            .join(".agents")
            .join("skills")
            .join(skill_name);
        let result = binary::install_binary_from_github(
            &source.source,
            binary_name,
            source.rev.as_deref(),
            &skill_dir,
        )?;

        // Update lockfile entry
        let locked = lockfile.find(skill_name).cloned();
        if let Some(mut entry) = locked {
            entry.binary_version = Some(result.version);
            entry.binary_checksum = Some(result.binary_checksum);
            lockfile.upsert(entry);
        }

        // Clean up old version if different
        if current_version != "unknown" && current_version != latest_version {
            let _ = binary::remove_binary_version(binary_name, &current_version);
        }

        updated_count += 1;
        println!("    Updated to v{}", p.info(&latest_version));
    }

    lockfile.write_to(&ctx.lockfile_path)?;

    if updated_count > 0 {
        println!(
            "{}",
            p.success(&format!("Updated {} skill(s).", updated_count))
        );
    } else {
        println!("All binary skills are up to date.");
    }

    Ok(())
}
