use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use ion_skill::manifest::ManifestOptions;
use ion_skill::migrate::{
    DiscoveredSkill, DiscoveryOrigin, MigrateOptions, ResolvedSkill, discover_from_directories,
    discover_from_lockfile,
};

pub fn run(from: Option<&str>, dry_run: bool) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let lockfile_path = from
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join("skills-lock.json"));

    // Discover skills
    let discovered = if lockfile_path.exists() {
        let skills = discover_from_lockfile(&lockfile_path)?;
        println!("Found skills-lock.json with {} skills.", skills.len());
        skills
    } else {
        println!("No skills-lock.json found, scanning directories...");
        let skills = discover_from_directories(&project_dir)?;
        if skills.is_empty() {
            println!("No skills found in .agents/skills/ or .claude/skills/.");
            return Ok(());
        }
        println!("Found {} skills in skill directories.", skills.len());
        skills
    };

    if discovered.is_empty() {
        println!("No skills to migrate.");
        return Ok(());
    }

    // Resolve each skill (prompt for missing info)
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut resolved = Vec::new();
    let mut skipped = Vec::new();

    for skill in &discovered {
        match resolve_skill(skill, &mut stdin_lock) {
            Some(r) => resolved.push(r),
            None => skipped.push(skill.name.clone()),
        }
    }

    if resolved.is_empty() {
        println!("No skills to migrate (all skipped).");
        print_skipped(&skipped);
        return Ok(());
    }

    // Show plan
    println!();
    if dry_run {
        println!("Dry run — would migrate {} skills:", resolved.len());
    } else {
        println!("Migrating {} skills...", resolved.len());
    }

    for skill in &resolved {
        let source_display = format_source(&skill.source);
        let rev_display = skill
            .rev
            .as_deref()
            .map(|r| format!(" @ {r}"))
            .unwrap_or_default();
        println!("  {} ({}{}) ...", skill.name, source_display, rev_display);
    }

    if dry_run {
        println!();
        println!("Dry run complete. No files were written.");
        print_skipped(&skipped);
        return Ok(());
    }

    // Execute migration
    let options = MigrateOptions {
        dry_run: false,
        manifest_options: ManifestOptions::default(),
    };

    let locked = ion_skill::migrate::migrate(&project_dir, &resolved, &options)?;

    // Print results
    for entry in &locked {
        let commit_display = entry
            .commit
            .as_deref()
            .map(|c| {
                if c.len() > 7 {
                    &c[..7]
                } else {
                    c
                }
            })
            .unwrap_or("(none)");
        println!("  {} ... fetched, commit {}", entry.name, commit_display);
    }

    println!();
    println!(
        "Written ion.toml with {} skills.",
        locked.len()
    );
    println!(
        "Written ion.lock with {} locked entries.",
        locked.len()
    );

    print_skipped(&skipped);

    println!("Done! You can now use `ion install`, `ion list`, etc.");
    Ok(())
}

fn resolve_skill(skill: &DiscoveredSkill, stdin: &mut impl BufRead) -> Option<ResolvedSkill> {
    let source = match &skill.source {
        Some(s) => s.clone(),
        None => {
            // Prompt for source
            print!(
                "Skill '{}' found in {} but source is unknown.\nEnter source (e.g., owner/repo/skill or git URL), or press Enter to skip:\n> ",
                skill.name,
                origin_label(&skill.origin),
            );
            io::stdout().flush().ok();

            let mut input = String::new();
            stdin.read_line(&mut input).ok()?;
            let input = input.trim();

            if input.is_empty() {
                return None;
            }

            match ion_skill::source::SkillSource::infer(input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("  Invalid source: {e}. Skipping '{}'.", skill.name);
                    return None;
                }
            }
        }
    };

    // Prompt for rev pinning
    print!(
        "Pin '{}' to a specific ref? (branch/tag/SHA, or Enter to use latest):\n> ",
        skill.name,
    );
    io::stdout().flush().ok();

    let mut rev_input = String::new();
    stdin.read_line(&mut rev_input).ok();
    let rev = rev_input.trim();
    let rev = if rev.is_empty() {
        None
    } else {
        Some(rev.to_string())
    };

    Some(ResolvedSkill {
        name: skill.name.clone(),
        source,
        rev,
    })
}

fn origin_label(origin: &DiscoveryOrigin) -> &'static str {
    match origin {
        DiscoveryOrigin::LockFile => "skills-lock.json",
        DiscoveryOrigin::AgentsDir => ".agents/skills/",
        DiscoveryOrigin::ClaudeDir => ".claude/skills/",
    }
}

fn format_source(source: &ion_skill::source::SkillSource) -> String {
    match &source.path {
        Some(path) => format!("{}/{}", source.source, path),
        None => source.source.clone(),
    }
}

fn print_skipped(skipped: &[String]) {
    if !skipped.is_empty() {
        println!();
        println!("Skipped {} skills (add manually with `ion add`):", skipped.len());
        for name in skipped {
            println!("  - {name}");
        }
    }
}
