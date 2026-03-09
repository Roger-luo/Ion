# JSON Agent Interface Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a global `--json` flag that gives agents/scripts a two-stage, non-interactive interface to every Ion command.

**Architecture:** Global `--json` flag on the `Cli` struct, threaded through command dispatch as a bool. A shared `json` output module provides the JSON envelope types and print helpers. Commands check the flag to decide between human output and structured JSON. Interactive prompts become two-stage: return options (exit 2) or execute (exit 0).

**Tech Stack:** serde + serde_json for serialization, clap for the global flag, existing crossterm/ratatui for human mode.

---

### Task 1: Add serde_json dependency and JSON output module

**Files:**
- Modify: `Cargo.toml` (root)
- Create: `src/json.rs`
- Modify: `src/main.rs` (add `mod json`)

**Step 1: Add serde_json to root Cargo.toml**

In `Cargo.toml`, add to `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Step 2: Create src/json.rs with envelope types and helpers**

```rust
use serde::Serialize;

/// Standard JSON response envelope.
#[derive(Serialize)]
#[serde(untagged)]
pub enum JsonResponse<T: Serialize> {
    Success {
        success: bool,
        data: T,
    },
    ActionRequired {
        success: bool,
        action_required: &'static str,
        data: T,
    },
    Error {
        success: bool,
        error: String,
    },
}

/// Print a success response and exit 0.
pub fn print_success<T: Serialize>(data: T) {
    let resp = JsonResponse::Success::<T> { success: true, data };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
}

/// Print an action-required response and exit 2.
pub fn print_action_required<T: Serialize>(action: &'static str, data: T) -> ! {
    let resp = JsonResponse::ActionRequired::<T> {
        success: false,
        action_required: action,
        data,
    };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    std::process::exit(2);
}

/// Print a JSON error and exit 1.
pub fn print_error(msg: &str) -> ! {
    let resp = JsonResponse::Error::<()> {
        success: false,
        error: msg.to_string(),
    };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    std::process::exit(1);
}
```

**Step 3: Register the module in main.rs**

Add `mod json;` alongside the existing `mod commands;` line in `src/main.rs`.

**Step 4: Build to verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/json.rs src/main.rs
git commit -m "feat: add JSON output module with envelope types"
```

---

### Task 2: Add global --json flag and thread through dispatch

**Files:**
- Modify: `src/main.rs`

**Step 1: Add `--json` flag to `Cli` struct**

In `src/main.rs`, modify the `Cli` struct:

```rust
#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager")]
struct Cli {
    /// Output results as JSON (for agents and scripts)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}
```

**Step 2: Update the dispatch in `main()` to pass `json` flag**

Update the `main()` function. First, update the error handler at the bottom:

```rust
fn main() {
    let cli = Cli::parse();
    let json = cli.json;

    let result = match cli.command {
        // ... all existing arms unchanged for now ...
    };

    if let Err(e) = result {
        if json {
            crate::json::print_error(&e.to_string());
        } else {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
```

Each command arm keeps its current form for now — we'll thread `json` into individual commands in later tasks.

**Step 3: Build and test the flag parses**

Run: `cargo build && ./target/debug/ion --json --help`
Expected: Help text shows `--json` flag. Compiles without errors.

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add global --json flag to CLI"
```

---

### Task 3: JSON output for `ion search` + make interactive the default

**Files:**
- Modify: `src/main.rs` (remove `--interactive` flag, pass `json`)
- Modify: `src/commands/search.rs`

**Step 1: Update Search args in main.rs**

Remove the `interactive` field from the `Search` variant in `Commands` enum. Add nothing — interactive is now the default, JSON is the opt-out.

```rust
Search {
    /// Search query (word or phrase)
    query: String,
    /// Include configured CLI agent in search
    #[arg(long)]
    agent: bool,
    /// Search only a specific source
    #[arg(long)]
    source: Option<String>,
    /// Max results per source
    #[arg(long, default_value = "50")]
    limit: usize,
    /// Enable verbose debug logging
    #[arg(long, short)]
    verbose: bool,
},
```

Update the dispatch arm:

```rust
Commands::Search { query, agent, source, limit, verbose } => {
    if verbose {
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }
    commands::search::run(&query, agent, json, source.as_deref(), limit)
}
```

**Step 2: Rewrite search.rs `run()` signature and logic**

Change `run()` to accept `json: bool` instead of `interactive: bool`:

```rust
use std::io::IsTerminal;

pub fn run(
    query: &str,
    agent: bool,
    json: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
    log::debug!("search starting: query={query:?}, agent={agent}, json={json}, source={source_filter:?}, limit={limit}");
    let config = GlobalConfig::load()?;
    let mut results = execute_search(&config, query, agent, source_filter, limit)?;

    if results.is_empty() {
        if json {
            crate::json::print_success(serde_json::json!([]));
            return Ok(());
        }
        println!("No results found for '{query}'.");
        return Ok(());
    }

    enrich_github_results(&mut results);

    if json {
        crate::json::print_success(&results);
        return Ok(());
    }

    // Human mode: TUI if TTY, otherwise plain text
    if std::io::stdout().is_terminal() {
        print_results(&results);
        pick_and_install(&results)?;
    } else {
        print_results(&results);
    }

    Ok(())
}
```

**Step 3: Write an integration test for JSON search output**

In `tests/search_integration.rs` (or a new file `tests/json_integration.rs`), add a test that verifies the `--json` flag produces valid JSON. Since search requires network, test with a source that will fail gracefully or test help output:

```rust
#[test]
fn search_json_flag_parses() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ion"))
        .args(["--json", "search", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --interactive flag should no longer appear
    assert!(!stdout.contains("--interactive"), "removed --interactive flag should not appear in help");
}
```

**Step 4: Run tests**

Run: `cargo test search_json`
Expected: PASS

**Step 5: Commit**

```bash
git add src/main.rs src/commands/search.rs tests/
git commit -m "feat: make search interactive by default, add --json output"
```

---

### Task 4: JSON output for `ion add` with `--allow-warnings` and `--skills`

**Files:**
- Modify: `src/main.rs` (add new flags to Add variant)
- Modify: `src/commands/add.rs`

**Step 1: Add new flags to Add variant in main.rs**

```rust
Add {
    /// Skill source (e.g., owner/repo/skill or git URL). Omit to install all from Ion.toml.
    source: Option<String>,
    /// Pin to a specific git ref (branch, tag, or commit SHA)
    #[arg(long)]
    rev: Option<String>,
    /// Install as a binary CLI skill from GitHub Releases
    #[arg(long)]
    bin: bool,
    /// Proceed despite validation warnings
    #[arg(long)]
    allow_warnings: bool,
    /// Comma-separated list of skills to install from a collection
    #[arg(long)]
    skills: Option<String>,
},
```

Update dispatch:

```rust
Commands::Add { source, rev, bin, allow_warnings, skills } => match source {
    Some(src) => commands::add::run(&src, rev.as_deref(), bin, json, allow_warnings, skills.as_deref()),
    None => commands::install::run(json, allow_warnings),
},
```

**Step 2: Update `add.rs` function signature**

```rust
pub fn run(
    source_str: &str,
    rev: Option<&str>,
    bin: bool,
    json: bool,
    allow_warnings: bool,
    skills_filter: Option<&str>,
) -> anyhow::Result<()> {
```

**Step 3: Add JSON handling for single-skill warnings**

Replace the `confirm_install_on_warnings()` calls (two locations — lines ~54 and ~84) with JSON-aware logic:

```rust
Err(SkillError::ValidationWarning { report, .. }) => {
    if json && !allow_warnings {
        // Two-stage: return warnings, agent must re-run with --allow-warnings
        crate::json::print_action_required("validation_warnings", serde_json::json!({
            "skill": name,
            "warnings": report.findings.iter().map(|f| serde_json::json!({
                "severity": f.severity.to_string(),
                "checker": f.checker,
                "message": f.message,
            })).collect::<Vec<_>>(),
        }));
    }
    if !json {
        print_validation_report(&name, &report);
        if !confirm_install_on_warnings()? {
            anyhow::bail!("Installation cancelled due to validation warnings.");
        }
    }
    // json + allow_warnings, or human confirmed: proceed
    let locked = installer.install_with_options(
        &name,
        &source,
        InstallValidationOptions {
            skip_validation: false,
            allow_warnings: true,
        },
    )?;
    return finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json);
}
```

**Step 4: Add JSON handling for collection install**

In `install_collection()`, update the signature to receive `json`, `allow_warnings`, and `skills_filter`:

```rust
fn install_collection(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    base_source: &SkillSource,
    source_str: &str,
    json: bool,
    allow_warnings: bool,
    skills_filter: Option<&str>,
) -> anyhow::Result<()> {
```

After discovering and validating skills, if `json` is true and no `skills_filter` provided, return the skill list:

```rust
// After Phase 1 validation...
if json && skills_filter.is_none() {
    let skills_data: Vec<_> = clean.iter().map(|e| serde_json::json!({
        "name": e.name, "status": "clean"
    })).chain(warned.iter().map(|(e, r)| serde_json::json!({
        "name": e.name, "status": "warnings", "warning_count": r.warning_count
    }))).chain(errored.iter().map(|(name, r)| serde_json::json!({
        "name": name, "status": "error", "error_count": r.error_count
    }))).collect();

    crate::json::print_action_required("skill_selection", serde_json::json!({
        "skills": skills_data
    }));
}
```

When `skills_filter` is provided, parse it and install only those:

```rust
if let Some(filter) = skills_filter {
    let selected_names: Vec<&str> = filter.split(',').map(|s| s.trim()).collect();
    // Install only skills matching selected_names from clean + warned (with allow_warnings)
    // Skip errored skills even if selected
}
```

**Step 5: Update `finish_single_install` for JSON output**

Add `json: bool` parameter. When `json` is true, emit JSON success instead of println:

```rust
fn finish_single_install(
    ctx: &ProjectContext,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
    json: bool,
) -> anyhow::Result<()> {
    // ... existing file operations (gitignore, manifest, lockfile) ...

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "installed_to": format!(".agents/skills/{name}/"),
            "targets": merged_options.targets.keys().collect::<Vec<_>>(),
        }));
    } else {
        // ... existing println! output ...
    }
    Ok(())
}
```

**Step 6: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 7: Commit**

```bash
git add src/main.rs src/commands/add.rs
git commit -m "feat: add --json two-stage flow for ion add with --allow-warnings and --skills"
```

---

### Task 5: JSON output for `ion add` (install-all) via `install.rs`

**Files:**
- Modify: `src/commands/install.rs`

**Step 1: Update signature**

```rust
pub fn run(json: bool, allow_warnings: bool) -> anyhow::Result<()> {
```

**Step 2: Add JSON two-stage flow for warned skills**

After Phase 1 validation, when `json` is true and there are warned skills and `!allow_warnings`:

```rust
if json && !warned.is_empty() && !allow_warnings {
    let skills_data: Vec<_> = clean.iter().map(|e| serde_json::json!({
        "name": e.name, "status": "clean"
    })).chain(warned.iter().map(|(e, r)| serde_json::json!({
        "name": e.name, "status": "warnings", "warning_count": r.warning_count
    }))).chain(errored.iter().map(|(name, _)| serde_json::json!({
        "name": name, "status": "error"
    }))).collect();

    crate::json::print_action_required("validation_warnings", serde_json::json!({
        "skills": skills_data
    }));
}
```

When `json && allow_warnings`: skip the interactive `select_warned_skills()` and install all warned skills.

When `json` and no warnings: install all clean skills and emit success JSON.

**Step 3: Add JSON success output at end**

```rust
if json {
    let installed: Vec<&str> = /* collect names of installed skills */;
    crate::json::print_success(serde_json::json!({
        "installed": installed,
        "skipped": errored.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
    }));
}
```

**Step 4: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 5: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: add --json two-stage flow for install-all command"
```

---

### Task 6: JSON output for `ion remove`

**Files:**
- Modify: `src/commands/remove.rs`
- Modify: `src/main.rs` (thread `json` to remove)

**Step 1: Update dispatch in main.rs**

```rust
Commands::Remove { name, yes } => commands::remove::run(&name, yes, json),
```

**Step 2: Update remove.rs signature and add JSON two-stage**

```rust
pub fn run(name: &str, yes: bool, json: bool) -> anyhow::Result<()> {
```

After determining `skills_to_remove`, add JSON handling:

```rust
if json && !yes {
    crate::json::print_action_required("confirm_removal", serde_json::json!({
        "skills": skills_to_remove,
    }));
}

// If json && yes: proceed with removal, skip prompt
if !json && !yes {
    // existing interactive prompt
}
```

At the end, emit JSON success:

```rust
if json {
    crate::json::print_success(serde_json::json!({
        "removed": skills_to_remove,
    }));
}
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/main.rs src/commands/remove.rs
git commit -m "feat: add --json two-stage flow for ion remove"
```

---

### Task 7: JSON output for `ion project init`

**Files:**
- Modify: `src/main.rs` (thread `json`)
- Modify: `src/commands/init.rs`

**Step 1: Update dispatch in main.rs**

```rust
ProjectCommands::Init { target, force } => commands::init::run(&target, force, json),
```

**Step 2: Update init.rs**

```rust
pub fn run(targets: &[String], force: bool, json: bool) -> anyhow::Result<()> {
```

When `json` and no `--target` flags:

```rust
if json && targets.is_empty() {
    // Return available targets + auto-detected ones
    let detected: Vec<_> = KNOWN_TARGETS.iter()
        .map(|(name, dir, path)| {
            let exists = ctx.project_dir.join(dir).exists();
            serde_json::json!({"name": name, "path": path, "detected": exists})
        })
        .collect();

    crate::json::print_action_required("target_selection", serde_json::json!({
        "available_targets": detected,
        "hint": "Re-run with --target flags to select targets",
    }));
}
```

When `json` with `--target` flags, emit JSON success at the end:

```rust
if json {
    crate::json::print_success(serde_json::json!({
        "targets": resolved,
        "manifest": "Ion.toml",
    }));
} else {
    // existing println! output
}
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/main.rs src/commands/init.rs
git commit -m "feat: add --json two-stage flow for ion project init"
```

---

### Task 8: JSON output for `ion config`

**Files:**
- Modify: `src/main.rs` (thread `json`)
- Modify: `src/commands/config.rs`

**Step 1: Update dispatch in main.rs**

```rust
Commands::Config { action } => commands::config::run(action, json),
```

**Step 2: Update config.rs**

```rust
pub fn run(action: Option<ConfigAction>, json: bool) -> anyhow::Result<()> {
    match action {
        None if json => anyhow::bail!("Interactive config editor not available in --json mode. Use 'ion config get/set/list'."),
        None => run_interactive(),
        Some(ConfigAction::Get { key, project }) => run_get(&key, project, json),
        Some(ConfigAction::Set { key, value, project }) => run_set(&key, &value, project, json),
        Some(ConfigAction::List { project }) => run_list(project, json),
    }
}
```

For `run_get` with JSON: output `{"success": true, "data": {"key": key, "value": value}}`.
For `run_set` with JSON: output `{"success": true, "data": {"key": key, "value": value}}`.
For `run_list` with JSON: output `{"success": true, "data": {"values": {key: value, ...}}}`.

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/main.rs src/commands/config.rs
git commit -m "feat: add --json output for ion config get/set/list"
```

---

### Task 9: JSON output for remaining commands

**Files:**
- Modify: `src/main.rs` (thread `json` to all remaining commands)
- Modify: `src/commands/list.rs`
- Modify: `src/commands/info.rs`
- Modify: `src/commands/update.rs`
- Modify: `src/commands/self_cmd.rs`
- Modify: `src/commands/validate.rs` (the `skill validate` command)
- Modify: `src/commands/gc.rs`
- Modify: `src/commands/new.rs`
- Modify: `src/commands/eject.rs`
- Modify: `src/commands/link.rs`

**Step 1: Thread `json` from main.rs dispatch to all command `run()` functions**

Update every dispatch arm to pass `json`. For commands that don't need special JSON handling yet, just add the parameter and use it for success output.

**Step 2: `list.rs` — output skill list as JSON**

Add `json: bool` parameter. When JSON, serialize the skill entries:

```rust
if json {
    let skills: Vec<_> = /* collect skill info structs */;
    crate::json::print_success(serde_json::json!({"skills": skills}));
    return Ok(());
}
```

**Step 3: `info.rs` — output skill info as JSON**

When JSON, serialize the skill metadata and source info as a JSON object.

**Step 4: `update.rs` — output update results as JSON**

When JSON, collect update results and emit at the end:

```rust
crate::json::print_success(serde_json::json!({
    "updated": updated_skills,
    "up_to_date": up_to_date_skills,
    "skipped": skipped_skills,
}));
```

**Step 5: `self_cmd.rs` — JSON for info/check/update**

- `info()`: `{"version": "...", "target": "...", "exe": "..."}`
- `check()`: `{"installed": "...", "latest": "...", "update_available": bool}`
- `update()`: `{"version": "...", "exe": "..."}`

**Step 6: `validate.rs` (skill validate) — JSON output**

Output findings as structured data:

```rust
crate::json::print_success(serde_json::json!({
    "skills": validated_skills,
    "errors": error_count,
    "warnings": warning_count,
}));
```

**Step 7: `gc.rs` — JSON output**

```rust
crate::json::print_success(serde_json::json!({
    "cleaned": count,
    "dry_run": dry_run,
}));
```

**Step 8: `new.rs` — JSON output**

When `json` and name is needed, bail with an error asking for `--name` flag (future: add `--name` flag to `skill new`). When JSON, emit success with created path.

Note: `new.rs` has an interactive `prompt_skill_name()` for local skills. In JSON mode, this should error with a message telling the agent to provide the name via a flag. Consider adding `--name` flag to `SkillCommands::New` in `main.rs`.

**Step 9: `eject.rs` and `link.rs` — JSON output**

Simple success output with skill name and paths.

**Step 10: Build and run all tests**

Run: `cargo build && cargo test`
Expected: All tests pass.

**Step 11: Commit**

```bash
git add src/main.rs src/commands/
git commit -m "feat: add --json output for all remaining commands"
```

---

### Task 10: Add `--name` flag to `ion skill new` for non-interactive use

**Files:**
- Modify: `src/main.rs` (add `--name` to SkillCommands::New)
- Modify: `src/commands/new.rs`

**Step 1: Add `--name` flag**

In `SkillCommands::New`:

```rust
New {
    /// Target directory (default: current directory)
    #[arg(long)]
    path: Option<String>,
    /// Set the project skills directory (persisted to Ion.toml)
    #[arg(long)]
    dir: Option<String>,
    /// Skill name (required in --json mode for local skills)
    #[arg(long)]
    name: Option<String>,
    /// Also run `cargo init --bin` to scaffold a Rust CLI project
    #[arg(long)]
    bin: bool,
    /// Create a multi-skill collection with a skills/ directory
    #[arg(long)]
    collection: bool,
    /// Overwrite existing files
    #[arg(long)]
    force: bool,
},
```

**Step 2: Update new.rs**

Update `run()` signature to accept `name: Option<&str>` and `json: bool`. When a name is provided, use it instead of prompting. When `json` and no name, bail with error.

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/main.rs src/commands/new.rs
git commit -m "feat: add --name flag to skill new for non-interactive use"
```

---

### Task 11: Integration tests for JSON mode

**Files:**
- Create: `tests/json_integration.rs`

**Step 1: Write integration tests**

```rust
use std::process::Command;

fn ion() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn json_flag_appears_in_help() {
    let output = ion().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--json"));
}

#[test]
fn json_error_is_structured() {
    // Run a command that will fail (e.g., remove nonexistent skill)
    let output = ion()
        .args(["--json", "remove", "--yes", "nonexistent-skill-xyz"])
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"].is_string());
}

#[test]
fn json_skill_list_empty_project() {
    let dir = tempfile::tempdir().unwrap();
    // Create minimal Ion.toml
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
}

#[test]
fn json_remove_without_yes_returns_action_required() {
    let dir = tempfile::tempdir().unwrap();
    // Create Ion.toml with a dummy skill
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n"
    ).unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();

    let output = ion()
        .args(["--json", "remove", "test-skill"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["action_required"], "confirm_removal");
}

#[test]
fn json_init_without_targets_returns_action_required() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "project", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["action_required"], "target_selection");
}

#[test]
fn json_init_with_targets_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "project", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
}

#[test]
fn interactive_flag_removed_from_search() {
    let output = ion()
        .args(["search", "--interactive", "test"])
        .output()
        .unwrap();
    // Should fail because --interactive no longer exists
    assert!(!output.status.success());
}
```

**Step 2: Run tests**

Run: `cargo test --test json_integration`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add tests/json_integration.rs
git commit -m "test: add integration tests for --json mode"
```

---

### Task 12: Update `--json` behavior in `print_no_targets_hint`

**Files:**
- Modify: `src/commands/init.rs`

**Step 1: Make `print_no_targets_hint` json-aware**

The hint is called from `add.rs` and `link.rs` after installs. In JSON mode it should be suppressed (the JSON response already contains target info).

Update the function to accept `json: bool`:

```rust
pub fn print_no_targets_hint(merged_options: &ion_skill::manifest::ManifestOptions, p: &crate::style::Paint, json: bool) {
    if json { return; }
    if merged_options.targets.is_empty() {
        println!();
        println!("  {}: skills are only installed to .agents/skills/ (the default location)", p.warn("hint"));
        println!("        To also install to .claude/skills/ or other tools, run: {}", p.bold("ion project init"));
    }
}
```

Update all callers in `add.rs`, `link.rs`, and `install.rs` to pass the `json` flag.

**Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add src/commands/init.rs src/commands/add.rs src/commands/link.rs src/commands/install.rs
git commit -m "refactor: suppress human hints in --json mode"
```

---

### Task 13: Final build, clippy, and test sweep

**Files:** None new — verification only.

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings.

**Step 3: Run fmt**

Run: `cargo fmt`
Expected: No changes (code already formatted).

**Step 4: Manual smoke test**

```bash
# JSON error
./target/debug/ion --json remove --yes nonexistent 2>&1
# JSON help
./target/debug/ion --json --help
# Verify --interactive removed
./target/debug/ion search --interactive test 2>&1 | head -1
```

**Step 5: Commit any fixups**

```bash
git add -A
git commit -m "chore: clippy and fmt fixes for JSON interface"
```
