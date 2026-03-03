# `ion init` & Target Discoverability Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an `ion init` command that configures target directories in `Ion.toml`, and show contextual hints during `ion add` when no targets are configured.

**Architecture:** New `src/commands/init.rs` command module with a known-targets lookup table, auto-detection of tool directories, interactive checkbox picker (raw stdin — no new deps), and `toml_edit`-based Ion.toml writing. A helper function `manifest_writer::write_targets` handles the TOML mutation. The hint is a simple conditional print at the end of `add.rs`.

**Tech Stack:** Rust, clap (CLI args), toml_edit (lossless TOML editing), raw stdin for interactive prompts.

---

### Task 1: Add `write_targets` to manifest_writer

Add a function to `crates/ion-skill/src/manifest_writer.rs` that writes `[options.targets]` into an existing or new Ion.toml, preserving existing content.

**Files:**
- Modify: `crates/ion-skill/src/manifest_writer.rs`

**Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `manifest_writer.rs`:

```rust
#[test]
fn write_targets_to_empty_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    let targets = std::collections::BTreeMap::from([
        ("claude".to_string(), ".claude/skills".to_string()),
    ]);
    write_targets(&path, &targets).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[skills]"), "existing content preserved");
    assert!(content.contains("[options]"));
    assert!(content.contains("claude"));
    assert!(content.contains(".claude/skills"));
}

#[test]
fn write_targets_preserves_existing_skills() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n").unwrap();

    let targets = std::collections::BTreeMap::from([
        ("claude".to_string(), ".claude/skills".to_string()),
    ]);
    write_targets(&path, &targets).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("brainstorming"));
    assert!(content.contains("claude"));
}

#[test]
fn write_targets_to_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");

    let targets = std::collections::BTreeMap::from([
        ("claude".to_string(), ".claude/skills".to_string()),
    ]);
    write_targets(&path, &targets).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[skills]"));
    assert!(content.contains("claude"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill write_targets`
Expected: FAIL — `write_targets` not found

**Step 3: Implement `write_targets`**

Add this function above the `skill_to_toml` function (after `remove_skill`):

```rust
/// Write target entries to an Ion.toml file's [options.targets] section.
/// Creates the file with a [skills] section if it doesn't exist.
/// Preserves all existing content.
pub fn write_targets(
    manifest_path: &Path,
    targets: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[options] is not a table".to_string()))?;

    options["targets"] = Item::Table(Table::new());
    let targets_table = options["targets"].as_table_mut().unwrap();
    for (k, v) in targets {
        targets_table[k.as_str()] = value(v.as_str());
    }

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill write_targets`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/manifest_writer.rs
git commit -m "feat: add write_targets to manifest_writer"
```

---

### Task 2: Add known-targets lookup table

Create a small module with the known targets and auto-detection logic.

**Files:**
- Create: `src/commands/init.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Write the failing test**

Create `src/commands/init.rs` with only test code at first:

```rust
use std::collections::BTreeMap;
use std::path::Path;

/// Known agent tool targets and their default skill directories.
const KNOWN_TARGETS: &[(&str, &str, &str)] = &[
    ("claude", ".claude", ".claude/skills"),
    ("cursor", ".cursor", ".cursor/skills"),
    ("windsurf", ".windsurf", ".windsurf/skills"),
];

/// Detect which known tool directories exist in the given project dir.
fn detect_targets(project_dir: &Path) -> Vec<(&'static str, &'static str)> {
    KNOWN_TARGETS
        .iter()
        .filter(|(_, dir, _)| project_dir.join(dir).is_dir())
        .map(|(name, _, path)| (*name, *path))
        .collect()
}

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

pub fn run(_targets: &[String], _force: bool) -> anyhow::Result<()> {
    todo!()
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
    fn detect_targets_finds_existing_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        let detected = detect_targets(dir.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0], ("claude", ".claude/skills"));
    }

    #[test]
    fn detect_targets_empty_when_no_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_targets(dir.path());
        assert!(detected.is_empty());
    }
}
```

**Step 2: Register the module**

Add `pub mod init;` to `src/commands/mod.rs` (after the `info` line).

**Step 3: Run tests to verify they pass**

Run: `cargo test init::tests`
Expected: 6 tests PASS (the unit tests work, `run()` is `todo!()` but not called)

**Step 4: Commit**

```bash
git add src/commands/init.rs src/commands/mod.rs
git commit -m "feat: add known-targets lookup and detection for ion init"
```

---

### Task 3: Wire `ion init` into clap CLI

Register the command in `main.rs` so it appears in `--help`.

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the Init variant to the Commands enum**

In `src/main.rs`, add after the `New { ... }` variant (line 97) and before `Config`:

```rust
    /// Initialize Ion.toml with agent tool targets
    Init {
        /// Configure specific targets (e.g. claude, cursor, or name:path)
        #[arg(long, short = 't')]
        target: Vec<String>,
        /// Overwrite existing [options.targets] without prompting
        #[arg(long)]
        force: bool,
    },
```

**Step 2: Add the match arm**

In the `main()` match block, add before the `Config` arm:

```rust
        Commands::Init { target, force } => commands::init::run(&target, force),
```

**Step 3: Run the help test**

Run: `cargo test help_shows_all_commands`
Expected: PASS (this test already expects "init" in the help output)

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: register ion init command in CLI"
```

---

### Task 4: Implement `ion init` — flag mode (non-interactive)

Implement the `run()` function for the flag-driven path first (simpler, no stdin).

**Files:**
- Modify: `src/commands/init.rs`

**Step 1: Write the failing integration test**

Add to `tests/integration.rs`:

```rust
#[test]
fn init_creates_manifest_with_target_flag() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "failed: stdout={stdout}\nstderr={stderr}");

    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("[skills]"));
    assert!(manifest.contains("claude"));
    assert!(manifest.contains(".claude/skills"));
}

#[test]
fn init_with_custom_target_path() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude:.claude/commands/skills"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains(".claude/commands/skills"));
}

#[test]
fn init_preserves_existing_skills() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("brainstorming"), "existing skills preserved");
    assert!(manifest.contains("claude"), "target added");
}

#[test]
fn init_errors_when_targets_exist_without_force() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options]\n[options.targets]\nclaude = \".claude/skills\"\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "cursor"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail without --force");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already") || stderr.contains("--force"));
}

#[test]
fn init_force_overwrites_existing_targets() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options]\n[options.targets]\nclaude = \".claude/skills\"\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "cursor", "--force"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("cursor"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test init_creates_manifest`
Expected: FAIL — `run()` is `todo!()`

**Step 3: Implement `run()` in `src/commands/init.rs`**

Replace the `todo!()` in `run()` with the full implementation. Also add the necessary imports at the top:

```rust
use std::collections::BTreeMap;
use std::path::Path;

use crate::context::ProjectContext;
use ion_skill::manifest_writer;

const KNOWN_TARGETS: &[(&str, &str, &str)] = &[
    ("claude", ".claude", ".claude/skills"),
    ("cursor", ".cursor", ".cursor/skills"),
    ("windsurf", ".windsurf", ".windsurf/skills"),
];

fn detect_targets(project_dir: &Path) -> Vec<(&'static str, &'static str)> {
    KNOWN_TARGETS
        .iter()
        .filter(|(_, dir, _)| project_dir.join(dir).is_dir())
        .map(|(name, _, path)| (*name, *path))
        .collect()
}

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

fn rename_legacy_files(project_dir: &Path) -> anyhow::Result<()> {
    let old_manifest = project_dir.join("ion.toml");
    let new_manifest = project_dir.join("Ion.toml");
    let old_lock = project_dir.join("ion.lock");
    let new_lock = project_dir.join("Ion.lock");

    if old_manifest.exists() && new_manifest.exists() {
        anyhow::bail!(
            "Both ion.toml and Ion.toml found. Please remove one before running init."
        );
    }
    if old_manifest.exists() {
        std::fs::rename(&old_manifest, &new_manifest)?;
        println!("Renamed ion.toml → Ion.toml");
    }
    if old_lock.exists() && !new_lock.exists() {
        std::fs::rename(&old_lock, &new_lock)?;
        println!("Renamed ion.lock → Ion.lock");
    }
    Ok(())
}

pub fn run(targets: &[String], force: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

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
        // Interactive mode
        select_targets_interactive(&ctx.project_dir)?
    };

    // Write targets to Ion.toml
    manifest_writer::write_targets(&ctx.manifest_path, &resolved)?;

    if resolved.is_empty() {
        println!("Created Ion.toml");
    } else {
        println!("Created Ion.toml with {} target(s):", resolved.len());
        for (name, path) in &resolved {
            println!("  {name} → {path}");
        }
    }

    Ok(())
}
```

Note: `select_targets_interactive` is implemented in the next task. For now, add a placeholder:

```rust
fn select_targets_interactive(_project_dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    // Placeholder — implemented in Task 5
    Ok(BTreeMap::new())
}
```

**Step 4: Run integration tests**

Run: `cargo test init_creates_manifest init_with_custom init_preserves init_errors init_force`
Expected: All 5 PASS

**Step 5: Commit**

```bash
git add src/commands/init.rs tests/integration.rs
git commit -m "feat: implement ion init flag mode"
```

---

### Task 5: Implement `ion init` — interactive mode

Replace the `select_targets_interactive` placeholder with a stdin-based picker.

**Files:**
- Modify: `src/commands/init.rs`

**Step 1: Write the failing integration test**

Add to `tests/integration.rs`:

```rust
#[test]
fn init_interactive_detects_and_creates_targets() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".claude")).unwrap();

    // Simulate user pressing Enter (accept defaults)
    let mut child = ion_cmd()
        .args(["init"])
        .current_dir(project.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Send empty line to accept detected defaults
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    let output = child.wait_with_output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "failed: stdout={stdout}\nstderr={stderr}");
    assert!(project.path().join("Ion.toml").exists());
}
```

Add `use std::process::Stdio;` and `use std::io::Write;` at the top of the test file if not already present.

**Step 2: Run test to verify it fails**

Run: `cargo test init_interactive`
Expected: FAIL (placeholder returns empty map, may not match expected behavior)

**Step 3: Implement `select_targets_interactive`**

Replace the placeholder in `src/commands/init.rs`:

```rust
fn select_targets_interactive(project_dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    use std::io::Write;

    let detected = detect_targets(project_dir);
    if !detected.is_empty() {
        let names: Vec<&str> = detected.iter().map(|(n, _)| *n).collect();
        println!("Detected: {}", names.join(", "));
        println!();
    }

    println!("Which tools do you use? (comma-separated, or press Enter for detected)");
    for (name, _, path) in KNOWN_TARGETS {
        let marker = if detected.iter().any(|(n, _)| n == name) {
            "*"
        } else {
            " "
        };
        println!("  [{marker}] {name} ({path})");
    }
    print!("> ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();

    let mut targets = BTreeMap::new();

    if answer.is_empty() {
        // Accept detected defaults
        for (name, path) in &detected {
            targets.insert(name.to_string(), path.to_string());
        }
    } else {
        // Parse comma-separated list
        for item in answer.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (name, path) = parse_target_flag(item)?;
            targets.insert(name, path);
        }
    }

    Ok(targets)
}
```

**Step 4: Run tests**

Run: `cargo test init_interactive`
Expected: PASS

Also run all init tests: `cargo test init_`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/commands/init.rs tests/integration.rs
git commit -m "feat: implement ion init interactive mode"
```

---

### Task 6: Handle legacy `ion.toml` rename

**Files:**
- Modify: `src/commands/init.rs` (already implemented in Task 4's `rename_legacy_files`)

**Step 1: Write the failing integration test**

Add to `tests/integration.rs`:

```rust
#[test]
fn init_renames_legacy_lowercase_files() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("ion.toml"),
        "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n",
    ).unwrap();
    std::fs::write(
        project.path().join("ion.lock"),
        "version = 1\n\n[skills]\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "failed: stdout={stdout}\nstderr={stderr}");

    // Legacy files renamed
    assert!(!project.path().join("ion.toml").exists());
    assert!(!project.path().join("ion.lock").exists());
    assert!(project.path().join("Ion.toml").exists());
    assert!(project.path().join("Ion.lock").exists());

    // Content preserved + target added
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("brainstorming"));
    assert!(manifest.contains("claude"));

    // Output mentions rename
    assert!(stdout.contains("Renamed"));
}

#[test]
fn init_errors_when_both_legacy_and_new_exist() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("ion.toml"), "[skills]\n").unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Both ion.toml and Ion.toml"));
}
```

**Step 2: Run tests**

Run: `cargo test init_renames init_errors_when_both`
Expected: PASS (logic is already in `rename_legacy_files` from Task 4)

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for legacy file rename in ion init"
```

---

### Task 7: Add contextual hint to `ion add`

Show a hint when no targets are configured after a successful install.

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Write the failing integration test**

Add to `tests/integration.rs`:

```rust
#[test]
fn add_shows_hint_when_no_targets_configured() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill to add (avoids network)
    let skill_dir = project.path().join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\n---\nA test skill.\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["link", skill_dir.to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "failed: stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ion init"),
        "should show hint about ion init when no targets configured. stdout: {stdout}"
    );
}
```

Note: We use `link` here since it's local (no network). The hint should appear in `link` too — or we can add it to both `add` and `link`. Let's add it to a shared helper.

**Step 2: Run test to verify it fails**

Run: `cargo test add_shows_hint`
Expected: FAIL — no hint printed

**Step 3: Add the hint to `add.rs` and `link.rs`**

Add a helper function to `src/commands/init.rs` (public, reusable):

```rust
/// Print a hint if no targets are configured, suggesting `ion init`.
pub fn print_no_targets_hint(merged_options: &ion_skill::manifest::ManifestOptions) {
    if merged_options.targets.is_empty() {
        println!();
        println!("  hint: skills are only installed to .agents/skills/ (the default location)");
        println!("        To also install to .claude/skills/ or other tools, run: ion init");
    }
}
```

In `src/commands/add.rs`, in `finish_single_install` — add after the `println!("Done!");` line (line 194):

```rust
    crate::commands::init::print_no_targets_hint(merged_options);
```

Also in `install_collection` — add after `println!("Done!");` (line 159):

```rust
    crate::commands::init::print_no_targets_hint(merged_options);
```

In `src/commands/link.rs`, add the same call after the final output line.

**Step 4: Run tests**

Run: `cargo test add_shows_hint`
Expected: PASS

Run all tests: `cargo test`
Expected: All PASS

**Step 5: Commit**

```bash
git add src/commands/init.rs src/commands/add.rs src/commands/link.rs tests/integration.rs
git commit -m "feat: show hint about ion init when no targets configured"
```

---

### Task 8: Final verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Manual smoke test**

```bash
cd $(mktemp -d)
mkdir .claude
ion init
# Should detect .claude, prompt interactively, create Ion.toml

cd $(mktemp -d)
ion init --target claude --target cursor
cat Ion.toml
# Should show [options.targets] with both entries

cd $(mktemp -d)
ion init --target bad:///absolute
# Should error about absolute paths
```

**Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 4: Commit any fixes from verification**

```bash
git add -A
git commit -m "chore: final cleanup for ion init"
```
