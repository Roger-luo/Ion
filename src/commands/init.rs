use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::Path;

use crate::context::WorkspaceContext;
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

/// Detect likely built-in AGENTS.md template from project files.
fn detect_builtin_template(dir: &Path) -> Option<&'static str> {
    let has_cargo = dir.join("Cargo.toml").exists();
    let has_python = dir.join("pyproject.toml").exists()
        || dir.join("setup.py").exists()
        || dir.join("requirements.txt").exists();
    let has_julia = dir.join("Project.toml").exists();
    let has_typescript = dir.join("tsconfig.json").exists()
        || (dir.join("package.json").exists()
            && (dir.join("tsconfig.json").exists()
                || dir.join("src").join("index.ts").exists()
                || dir.join("index.ts").exists()));
    match (has_cargo, has_python, has_julia, has_typescript) {
        (true, true, _, _) => Some("rust+python"),
        (true, false, _, _) => Some("rust"),
        (false, true, _, _) => Some("python"),
        (false, false, true, _) => Some("julia"),
        (false, false, false, true) => Some("typescript"),
        _ => None,
    }
}

/// Prompt the user to set up an AGENTS.md template. Returns the chosen template
/// name, or `None` if the user declined or the prompt was skipped.
fn prompt_agents_template(dir: &Path) -> anyhow::Result<Option<String>> {
    if !std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let detected = detect_builtin_template(dir);
    match detected {
        Some(name) => {
            print!("  Set up AGENTS.md? [Y/n] (template: {name}) ");
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let answer = line.trim();
            if answer.is_empty() || answer.eq_ignore_ascii_case("y") {
                Ok(Some(name.to_string()))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub fn run(
    targets: &[String],
    force: bool,
    json: bool,
    project_flags: &[String],
) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let p = ws.paint();

    // Init always operates on CWD — even if we're inside a workspace,
    // the intent is to create/update Ion.toml at the current directory.
    let cwd = std::env::current_dir()?;
    let project = ion_skill::workspace::Project::new(cwd);

    // Check for existing manifest before any migration (case-exact check for
    // case-insensitive filesystems like macOS HFS+/APFS)
    let manifest_existed = dir_has_exact_name(&project.dir, "Ion.toml");

    // Handle legacy lowercase files
    rename_legacy_files(&project.dir)?;

    // If manifest already existed (not from legacy rename), require --force
    if manifest_existed && !force {
        anyhow::bail!("Ion.toml already exists. Use --force to overwrite.");
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
                let exists = project.dir.join(dir).exists();
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
        match select_targets_interactive(&project.dir)? {
            Some(targets) => targets,
            None => return Ok(()),
        }
    };

    // Write targets to Ion.toml
    manifest_writer::write_targets(&project.manifest_path, &resolved)?;

    // Install the built-in ion-cli skill so agents can discover Ion's JSON interface
    let manifest = project.manifest_or_empty()?;
    let merged_options = ws.merged_options_for(&project)?;
    ws.ensure_builtin_skill(&project, &merged_options);

    // Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
    if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&project.dir, &merged_options.targets)
    {
        log::warn!("Failed to create agent symlinks: {e}");
    }

    // Deploy agents-update skill if [agents] template is configured
    if manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .is_some()
        && let Err(e) =
            crate::commands::agents::deploy_agents_update_skill(&project, &merged_options)
    {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }

    // Auto-register in workspace if we're inside one
    if ws.is_workspace() {
        let root = ws.root_project();
        if let Ok(relative) = project.dir.strip_prefix(&root.dir) {
            let member_path = relative.display().to_string();
            if !member_path.is_empty() {
                // Check if already registered
                let root_manifest = root.manifest_or_empty()?;
                let already_member = root_manifest
                    .workspace
                    .as_ref()
                    .map(|w| w.members.contains(&member_path))
                    .unwrap_or(false);
                if !already_member {
                    ion_skill::manifest_writer::add_workspace_member(
                        &root.manifest_path,
                        &member_path,
                    )?;
                    if !json {
                        println!("  {} as workspace member", p.success("Registered"));
                    }
                }
            }
        }
    }

    // Offer inline AGENTS.md setup on fresh init (non-json, interactive, no existing AGENTS.md)
    let agents_template_used = if !json && !manifest_existed {
        let agents_md_path = project.dir.join("AGENTS.md");
        if !agents_md_path.exists() {
            match prompt_agents_template(&project.dir)? {
                Some(name) => {
                    let source = format!("builtin:{name}");
                    match crate::commands::agents::init(&source, None, None, false, project_flags) {
                        Ok(()) => Some(name),
                        Err(e) => {
                            eprintln!("  warning: AGENTS.md setup failed: {e}");
                            None
                        }
                    }
                }
                None => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    if json {
        crate::json::print_success(serde_json::json!({
            "targets": resolved,
            "manifest": "Ion.toml",
            "agents_template": agents_template_used,
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

    #[test]
    fn detect_typescript_via_tsconfig() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_builtin_template(dir.path()), Some("typescript"));
    }

    #[test]
    fn detect_typescript_via_index_ts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("index.ts"), "").unwrap();
        assert_eq!(detect_builtin_template(dir.path()), Some("typescript"));
    }

    #[test]
    fn detect_rust_takes_priority_over_typescript() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_builtin_template(dir.path()), Some("rust"));
    }
}
