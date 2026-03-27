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

/// Print a hint when the user tries to configure a target for Codex.
fn print_codex_hint(p: &crate::style::Paint) {
    println!(
        "  {}: Codex uses the default .agents/ directory — no extra target configuration needed.",
        p.warn("hint")
    );
}

/// Parse a --target flag value. Accepts "name" (uses lookup) or "name:path".
fn parse_target_flag(flag: &str) -> anyhow::Result<(String, String)> {
    if let Some((name, path)) = flag.split_once(':') {
        if Path::new(path).is_absolute() {
            anyhow::bail!("Target paths must be relative to the project directory: {path}");
        }
        Ok((name.to_string(), path.to_string()))
    } else if flag.eq_ignore_ascii_case("codex") {
        anyhow::bail!(
            "Codex uses the default .agents/ directory — no extra target configuration needed."
        )
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
        anyhow::bail!("Both ion.toml and Ion.toml found. Please remove one before running init.");
    }
    if has_old_manifest {
        std::fs::rename(project_dir.join("ion.toml"), project_dir.join("Ion.toml"))?;
        println!("Renamed ion.toml → Ion.toml");
    }
    if has_old_lock && !has_new_lock {
        std::fs::rename(project_dir.join("ion.lock"), project_dir.join("Ion.lock"))?;
        println!("Renamed ion.lock → Ion.lock");
    }
    Ok(())
}

fn select_targets_interactive(
    project_dir: &Path,
) -> anyhow::Result<Option<BTreeMap<String, String>>> {
    use crate::tui::init_select::run_init_select;

    run_init_select(project_dir)
}

/// Print a hint if no targets are configured, suggesting `ion init`.
pub fn print_no_targets_hint(
    merged_options: &ion_skill::manifest::ManifestOptions,
    p: &crate::style::Paint,
    json: bool,
) {
    if json {
        return;
    }
    if merged_options.targets.is_empty() {
        println!();
        println!(
            "  {}: skills are only installed to .agents/skills/ (the default location)",
            p.warn("hint")
        );
        println!(
            "        To also install to .claude/skills/ or other tools, run: {}",
            p.bold("ion init")
        );
    }
}

pub fn run(targets: &[String], force: bool, json: bool) -> anyhow::Result<()> {
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
    } else if json {
        let detected: Vec<_> = KNOWN_TARGETS
            .iter()
            .map(|(name, dir, path)| {
                let exists = ctx.project_dir.join(dir).exists();
                serde_json::json!({"name": name, "path": path, "detected": exists})
            })
            .collect();
        crate::json::print_action_required(
            "target_selection",
            serde_json::json!({
                "available_targets": detected,
                "hint": "Re-run with --target flags to select targets",
            }),
        );
    } else {
        match select_targets_interactive(&ctx.project_dir)? {
            Some(targets) => targets,
            None => return Ok(()),
        }
    };

    // Write targets to Ion.toml
    manifest_writer::write_targets(&ctx.manifest_path, &resolved)?;

    // Install the built-in ion-cli skill so agents can discover Ion's JSON interface
    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);
    ctx.ensure_builtin_skill(&merged_options);

    // Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
    if let Err(e) =
        ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets)
    {
        eprintln!("Warning: failed to create agent symlinks: {e}");
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "targets": resolved,
            "manifest": "Ion.toml",
        }));
        return Ok(());
    }

    if resolved.is_empty() {
        println!("{} Ion.toml", p.success("Created"));
    } else {
        println!(
            "{} Ion.toml with {} target(s):",
            p.success("Created"),
            p.bold(&resolved.len().to_string())
        );
        for (name, path) in &resolved {
            println!("  {} → {}", p.bold(name), p.info(path));
        }
    }

    // Show hint if any resolved target looks like codex
    if resolved.keys().any(|k| k.eq_ignore_ascii_case("codex")) {
        print_codex_hint(&p);
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

    #[test]
    fn parse_codex_target_shows_hint() {
        let err = parse_target_flag("codex").unwrap_err();
        assert!(
            err.to_string().contains(".agents/"),
            "should mention .agents/"
        );
    }

    #[test]
    fn parse_codex_case_insensitive() {
        assert!(parse_target_flag("Codex").is_err());
        assert!(parse_target_flag("CODEX").is_err());
    }

    #[test]
    fn parse_codex_with_custom_path_still_works() {
        let (name, path) = parse_target_flag("codex:custom/path").unwrap();
        assert_eq!(name, "codex");
        assert_eq!(path, "custom/path");
    }
}
