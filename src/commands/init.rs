use std::collections::BTreeMap;
use std::io::IsTerminal;
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

/// A suggested next step after `init`. Rendered to both the human text (with
/// its command emphasized) and the JSON `next` array (as a plain string) from
/// this single definition, so the two channels never drift.
struct NextStep {
    /// Imperative shown to the human, e.g. "Add your first skill".
    label: String,
    /// Exact command to run, if this step is a command.
    command: Option<String>,
    /// Extra parenthetical guidance (e.g. a discovery command).
    hint: Option<String>,
}

impl NextStep {
    /// Plain-string form emitted in the JSON `next` array. Agents get the exact
    /// command when there is one, otherwise the imperative label.
    fn to_json_string(&self) -> String {
        match &self.command {
            Some(cmd) => cmd.clone(),
            None => self.label.clone(),
        }
    }

    /// Styled single-line form for the human channel.
    fn to_human(&self, p: &crate::style::Paint) -> String {
        let mut line = self.label.clone();
        if let Some(cmd) = &self.command {
            line.push_str(&format!(" — {}", p.bold(cmd)));
        }
        if let Some(hint) = &self.hint {
            line.push_str(&format!(" ({hint})"));
        }
        line
    }
}

/// Build the ordered next-step list after a successful `init`, tailored to
/// what init just did. This is the single source of truth for both channels.
fn next_steps_after_init(created_agents_md: bool, real_skill_count: usize) -> Vec<NextStep> {
    let mut steps = Vec::new();

    // Whenever init created AGENTS.md from a template, the file has placeholder
    // sections — prompt the human/agent to fill them in before anything else.
    if created_agents_md {
        steps.push(NextStep {
            label: "Fill in AGENTS.md to describe this project, its build/test commands, and conventions".to_string(),
            command: None,
            hint: None,
        });
    }

    if real_skill_count > 0 {
        // Skills already declared in Ion.toml (re-init, or a cloned manifest) —
        // the next move is to install them.
        steps.push(NextStep {
            label: "Install the skills declared in Ion.toml".to_string(),
            command: Some("ion add".to_string()),
            hint: None,
        });
    } else {
        // Fresh project with no user skills yet — add the first one.
        steps.push(NextStep {
            label: "Add your first skill".to_string(),
            command: Some("ion add <source>".to_string()),
            hint: Some("browse skills with `ion search <query>`".to_string()),
        });
    }

    steps
}

/// Print the next-step guidance after a successful `init` (human channel).
fn print_next_steps(p: &crate::style::Paint, steps: &[NextStep]) {
    if steps.is_empty() {
        return;
    }
    println!();
    if steps.len() == 1 {
        println!("  {}: {}", p.bold("Next"), steps[0].to_human(p));
    } else {
        println!("  {}:", p.bold("Next steps"));
        for (i, step) in steps.iter().enumerate() {
            println!("    {}. {}", i + 1, step.to_human(p));
        }
    }
}

/// What `init` did about the project's AGENTS.md.
enum AgentsMdOutcome {
    /// `--no-agents`: skipped entirely.
    Disabled,
    /// Wrote a fresh AGENTS.md from a template.
    Created { template: String },
    /// Renamed an existing CLAUDE.md into AGENTS.md and linked it back.
    Migrated,
    /// AGENTS.md already existed; ensured agent tools link to it.
    Existing,
    /// AGENTS.md and CLAUDE.md both have content; left both untouched.
    ConflictSkipped { reason: String },
}

impl AgentsMdOutcome {
    fn to_json(&self) -> serde_json::Value {
        match self {
            AgentsMdOutcome::Disabled => serde_json::json!({"action": "disabled"}),
            AgentsMdOutcome::Created { template } => {
                serde_json::json!({"action": "created", "template": template})
            }
            AgentsMdOutcome::Migrated => {
                serde_json::json!({"action": "migrated", "from": "CLAUDE.md"})
            }
            AgentsMdOutcome::Existing => serde_json::json!({"action": "existing"}),
            AgentsMdOutcome::ConflictSkipped { reason } => {
                serde_json::json!({"action": "skipped", "reason": reason})
            }
        }
    }

    /// True when init wrote a fresh AGENTS.md from a template (so the user
    /// should be prompted to fill it in).
    fn created_from_template(&self) -> bool {
        matches!(self, AgentsMdOutcome::Created { .. })
    }
}

/// Ensure the project has an ion-managed AGENTS.md.
///
/// 1. Reconcile any existing CLAUDE.md into AGENTS.md (rename the unambiguous
///    case, replace a pointer with a symlink, or skip a genuine two-file
///    conflict). This runs silently and non-interactively; `init` narrates the
///    result itself so the messaging is consistent and init-appropriate.
/// 2. If AGENTS.md still doesn't exist, create it from a detected language
///    template, or a generic scaffold when no language matches.
fn ensure_agents_md(
    project: &ion_skill::workspace::Project,
    merged_options: &ion_skill::manifest::ManifestOptions,
    global_config: &ion_skill::config::GlobalConfig,
    p: &crate::style::Paint,
) -> anyhow::Result<AgentsMdOutcome> {
    use crate::commands::agents::{AgentsMdAction, migrate_claude_md};

    let agents_md = project.dir.join("AGENTS.md");

    // Reconcile an existing CLAUDE.md silently (json=true suppresses migrate's
    // own prompts and prints), auto-renaming the unambiguous case.
    let migration = migrate_claude_md(&project.dir, p, true, true, true)?;

    match migration {
        Some(AgentsMdAction::Renamed { .. }) => Ok(AgentsMdOutcome::Migrated),
        Some(AgentsMdAction::Symlinked) => Ok(AgentsMdOutcome::Existing),
        Some(AgentsMdAction::Skipped { .. }) => {
            if agents_md.exists() {
                // Genuine conflict — both files have real content. Use an
                // init-appropriate reason (migrate's mentions `--yes`).
                Ok(AgentsMdOutcome::ConflictSkipped {
                    reason: "AGENTS.md and CLAUDE.md both have content; kept both".to_string(),
                })
            } else {
                // e.g. a CLAUDE.md pointing at a nonexistent AGENTS.md — create one.
                create_agents_from_template(project, merged_options, global_config)
            }
        }
        None => {
            // No CLAUDE.md file to migrate.
            if agents_md.exists() {
                if let Err(e) =
                    ion_skill::agents::ensure_agent_symlinks(&project.dir, &merged_options.targets)
                {
                    log::warn!("Failed to create agent symlinks: {e}");
                }
                Ok(AgentsMdOutcome::Existing)
            } else {
                create_agents_from_template(project, merged_options, global_config)
            }
        }
    }
}

/// Create AGENTS.md from the best-matching built-in template (or the generic
/// fallback), wiring up `[agents]` tracking and agent symlinks.
fn create_agents_from_template(
    project: &ion_skill::workspace::Project,
    merged_options: &ion_skill::manifest::ManifestOptions,
    global_config: &ion_skill::config::GlobalConfig,
) -> anyhow::Result<AgentsMdOutcome> {
    let template = detect_builtin_template(&project.dir).unwrap_or("generic");
    let source = format!("builtin:{template}");
    let setup = crate::commands::agents::apply_template(
        project,
        merged_options,
        global_config,
        &source,
        None,
        None,
    )?;
    Ok(AgentsMdOutcome::Created {
        template: setup.template,
    })
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

pub fn run(
    targets: &[String],
    force: bool,
    no_agents: bool,
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
    } else if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        // No --target flags and no interactive terminal available (e.g. piped
        // stdin/stdout, CI, or an agent invoking `ion init` directly). The
        // interactive selector needs a real TTY in raw mode — entering it
        // here would crash with "Device not configured" (ENOTTY). Print the
        // available targets instead and ask the caller to pick explicitly,
        // mirroring the --json path's action_required above.
        println!("No target specified and no interactive terminal available.");
        println!();
        println!("Available targets:");
        // Mark targets whose directory already exists in this project. This
        // mirrors the `detected` flag the --json channel exposes, so a human
        // reading the plain-text list gets the same signal an agent does.
        let mut first_detected: Option<&str> = None;
        for (name, dir, path) in KNOWN_TARGETS {
            if project.dir.join(dir).exists() {
                if first_detected.is_none() {
                    first_detected = Some(name);
                }
                println!("  {name} -> {path} {}", p.dim("(detected)"));
            } else {
                println!("  {name} -> {path}");
            }
        }
        println!();
        // Prefer a detected target in the example so the recovery command lands
        // on the tool this repo is already set up for.
        let example = first_detected.unwrap_or("claude");
        println!("Re-run with --target <name> to select targets (e.g. --target {example}).");
        anyhow::bail!("no targets selected; re-run with --target <name>");
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

    // Re-deploy already-installed skills to the (newly) configured targets.
    // Without this, skills added via `ion add` before any target existed —
    // or before this target was configured — would stay invisible to that
    // agent tool until the user manually reran `ion add` for every skill.
    // `deploy()` only creates missing symlinks, so this is a cheap no-op for
    // skills that are already fully linked.
    if !resolved.is_empty() {
        let installer = ion_skill::installer::SkillInstaller::new(&project.dir, &merged_options);
        for name in manifest.skills.keys() {
            let skill_dir = installer.skill_dir(name);
            if skill_dir.exists() {
                let _ = installer.deploy(name, &skill_dir);
            }
        }
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

    // Ensure the project has an ion-managed AGENTS.md: migrate an existing
    // CLAUDE.md, or create one from a template. Runs in every channel; skipped
    // entirely with `--no-agents`.
    let agents_outcome = if no_agents {
        AgentsMdOutcome::Disabled
    } else {
        match ensure_agents_md(&project, &merged_options, &ws.global_config, &p) {
            Ok(outcome) => outcome,
            Err(e) => {
                if !json {
                    eprintln!("  {}: AGENTS.md setup failed: {e}", p.warn("warning"));
                }
                AgentsMdOutcome::Disabled
            }
        }
    };

    // Count user skills registered in the manifest (excluding Ion-managed
    // built-ins). This decides which next step to suggest: install existing
    // skills, or add the first one.
    let real_skill_count = manifest
        .skills
        .keys()
        .filter(|n| !is_managed_skill(n))
        .count();
    let next = next_steps_after_init(agents_outcome.created_from_template(), real_skill_count);

    if json {
        crate::json::print_success(serde_json::json!({
            "targets": resolved,
            "manifest": "Ion.toml",
            "agents_md": agents_outcome.to_json(),
            "next": next.iter().map(NextStep::to_json_string).collect::<Vec<_>>(),
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

    print_agents_outcome(&p, &agents_outcome);

    // Show hint if any resolved target looks like codex
    if resolved.keys().any(|k| k.eq_ignore_ascii_case("codex")) {
        print_codex_hint(&p);
    }

    print_next_steps(&p, &next);

    Ok(())
}

/// Whether a skill name is one Ion manages itself (not a user skill).
fn is_managed_skill(name: &str) -> bool {
    name == crate::builtin_skill::SKILL_NAME || name == "agents-update"
}

/// Print the human-channel summary line for what init did about AGENTS.md.
fn print_agents_outcome(p: &crate::style::Paint, outcome: &AgentsMdOutcome) {
    match outcome {
        AgentsMdOutcome::Disabled => {}
        AgentsMdOutcome::Created { template } => {
            let name = template.strip_prefix("builtin:").unwrap_or(template);
            println!(
                "  {} AGENTS.md from the {} template",
                p.success("Created"),
                p.bold(name)
            );
        }
        AgentsMdOutcome::Migrated => {
            println!(
                "  {} CLAUDE.md → AGENTS.md ({} now links to it)",
                p.success("Migrated"),
                p.dim("CLAUDE.md")
            );
        }
        AgentsMdOutcome::Existing => {
            println!(
                "  {} AGENTS.md (linked agent tools to it)",
                p.success("Using")
            );
        }
        AgentsMdOutcome::ConflictSkipped { reason } => {
            println!(
                "  {}: {} — merge them manually, or run {} to resolve interactively",
                p.warn("note"),
                reason,
                p.bold("ion migrate")
            );
        }
    }
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
