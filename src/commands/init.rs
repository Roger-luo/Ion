use std::collections::BTreeMap;
use std::path::Path;

use crate::context::ProjectContext;
use ion_skill::manifest_writer;

/// Known agent tool targets and their default skill directories.
const KNOWN_TARGETS: &[(&str, &str, &str)] = &[
    ("claude", ".claude", ".claude/skills"),
    ("cursor", ".cursor", ".cursor/skills"),
    ("windsurf", ".windsurf", ".windsurf/skills"),
];

/// Parse a --target flag value. Accepts "name" (uses lookup) or "name:path".
fn parse_target_flag(flag: &str) -> anyhow::Result<(String, String)> {
    if let Some((name, path)) = flag.split_once(':') {
        if Path::new(path).is_absolute() {
            anyhow::bail!("Target paths must be relative to the project directory: {path}");
        }
        Ok((name.to_string(), path.to_string()))
    } else {
        let known = KNOWN_TARGETS.iter().find(|(n, _, _)| *n == flag);
        match known {
            Some((name, _, path)) => Ok((name.to_string(), path.to_string())),
            None => anyhow::bail!(
                "Unknown target '{flag}'. Known targets: claude, cursor, windsurf. \
                 Use 'name:path' for custom targets."
            ),
        }
    }
}

/// Check whether a specific filename (exact case) exists in a directory
/// by scanning directory entries. This works correctly on case-insensitive
/// filesystems (e.g. macOS HFS+/APFS) where `Path::exists()` would match
/// both `ion.toml` and `Ion.toml`.
fn dir_has_exact_name(dir: &Path, name: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        entries
            .filter_map(|e| e.ok())
            .any(|e| e.file_name() == name)
    } else {
        false
    }
}

fn rename_legacy_files(project_dir: &Path) -> anyhow::Result<()> {
    let has_old_manifest = dir_has_exact_name(project_dir, "ion.toml");
    let has_new_manifest = dir_has_exact_name(project_dir, "Ion.toml");
    let has_old_lock = dir_has_exact_name(project_dir, "ion.lock");
    let has_new_lock = dir_has_exact_name(project_dir, "Ion.lock");

    if has_old_manifest && has_new_manifest {
        anyhow::bail!(
            "Both ion.toml and Ion.toml found. Please remove one before running init."
        );
    }
    if has_old_manifest {
        std::fs::rename(
            project_dir.join("ion.toml"),
            project_dir.join("Ion.toml"),
        )?;
        println!("Renamed ion.toml → Ion.toml");
    }
    if has_old_lock && !has_new_lock {
        std::fs::rename(
            project_dir.join("ion.lock"),
            project_dir.join("Ion.lock"),
        )?;
        println!("Renamed ion.lock → Ion.lock");
    }
    Ok(())
}

fn select_targets_interactive(project_dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    use crate::tui::init_select::run_init_select;

    match run_init_select(project_dir)? {
        Some(targets) => Ok(targets),
        None => anyhow::bail!("Cancelled"),
    }
}

/// Print a hint if no targets are configured, suggesting `ion init`.
pub fn print_no_targets_hint(merged_options: &ion_skill::manifest::ManifestOptions, p: &crate::style::Paint) {
    if merged_options.targets.is_empty() {
        println!();
        println!("  {}: skills are only installed to .agents/skills/ (the default location)", p.warn("hint"));
        println!("        To also install to .claude/skills/ or other tools, run: {}", p.bold("ion init"));
    }
}

pub fn run(targets: &[String], force: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = crate::style::Paint::new(&ctx.global_config);

    // Handle legacy lowercase files
    rename_legacy_files(&ctx.project_dir)?;

    // Check for existing targets (conflict detection)
    let manifest = ctx.manifest_or_empty()?;
    if !manifest.options.targets.is_empty() && !force {
        anyhow::bail!(
            "Ion.toml already has [options.targets] configured. Use --force to overwrite."
        );
    }

    // Resolve targets: flags take priority, otherwise interactive
    let resolved: BTreeMap<String, String> = if !targets.is_empty() {
        let mut map = BTreeMap::new();
        for flag in targets {
            let (name, path) = parse_target_flag(flag)?;
            map.insert(name, path);
        }
        map
    } else {
        select_targets_interactive(&ctx.project_dir)?
    };

    // Write targets to Ion.toml
    manifest_writer::write_targets(&ctx.manifest_path, &resolved)?;

    if resolved.is_empty() {
        println!("{} Ion.toml", p.success("Created"));
    } else {
        println!("{} Ion.toml with {} target(s):", p.success("Created"), p.bold(&resolved.len().to_string()));
        for (name, path) in &resolved {
            println!("  {} → {}", p.bold(name), p.info(path));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_target() {
        let (name, path) = parse_target_flag("claude").unwrap();
        assert_eq!(name, "claude");
        assert_eq!(path, ".claude/skills");
    }

    #[test]
    fn parse_custom_target() {
        let (name, path) = parse_target_flag("claude:.claude/commands/skills").unwrap();
        assert_eq!(name, "claude");
        assert_eq!(path, ".claude/commands/skills");
    }

    #[test]
    fn parse_unknown_target_is_error() {
        assert!(parse_target_flag("unknown").is_err());
    }

    #[test]
    fn parse_absolute_path_is_error() {
        assert!(parse_target_flag("foo:/absolute/path").is_err());
    }

}
