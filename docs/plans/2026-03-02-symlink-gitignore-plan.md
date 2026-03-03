# Symlink-Based Deployment + Per-Skill Gitignore Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace copy-based skill deployment with symlinks to a persistent global data directory, switch to per-skill gitignore entries, add `ion link` for local skills, and `ion gc` for stale repo cleanup.

**Architecture:** Change the installer to always use symlinks instead of copying. Remote skills symlink to a persistent `~/.local/share/ion/repos/` data directory. Local skills symlink to their project-relative path. Per-skill gitignore entries replace blanket directory ignores. A global registry tracks which projects use which repos for garbage collection.

**Tech Stack:** Rust, dirs crate (XDG paths), toml/serde (registry), clap (CLI)

---

### Task 1: Change storage location from cache to data dir

**Files:**
- Modify: `crates/ion-skill/src/installer.rs:12-17`

**Step 1: Write the failing test**

Add this test to the `#[cfg(test)] mod tests` block in `crates/ion-skill/src/installer.rs`:

```rust
#[test]
fn data_dir_uses_xdg_data_path() {
    let path = data_dir();
    let path_str = path.to_string_lossy();
    // Should NOT contain "Cache" or "cache" — must use data dir
    assert!(
        !path_str.to_lowercase().contains("cache"),
        "data_dir() should not use cache path, got: {path_str}"
    );
    assert!(path_str.contains("ion"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ion-skill data_dir_uses_xdg`
Expected: FAIL — `cache_dir()` contains "Cache"/"cache"

**Step 3: Implement the change**

In `crates/ion-skill/src/installer.rs`, rename `cache_dir()` to `data_dir()` and change the implementation:

```rust
/// Where ion stores cloned repositories persistently.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("repos")
}
```

Update all call sites within the file: `fetch_skill_base()` line 201 uses `cache_dir()` — change to `data_dir()`.

Also make `data_dir()` public (`pub fn`) since `registry.rs` and `gc.rs` will need it later.

Also make `hash_simple` public since `registry.rs` will need to compute repo hashes.

**Step 4: Run test to verify it passes**

Run: `cargo test -p ion-skill data_dir_uses_xdg`
Expected: PASS

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "refactor: change skill storage from cache to XDG data dir"
```

---

### Task 2: Add per-skill gitignore functions

**Files:**
- Modify: `crates/ion-skill/src/gitignore.rs`

**Step 1: Write the failing tests**

Add these tests to the `#[cfg(test)] mod tests` block in `crates/ion-skill/src/gitignore.rs`:

```rust
#[test]
fn add_skill_gitignore_entries_creates_section() {
    let project = tempfile::tempdir().unwrap();

    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(content.contains("# Managed by ion"));
    assert!(content.contains(".agents/skills/brainstorming"));
    assert!(content.contains(".claude/skills/brainstorming"));
}

#[test]
fn add_skill_gitignore_entries_is_idempotent() {
    let project = tempfile::tempdir().unwrap();

    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();
    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    let count = content.matches(".agents/skills/brainstorming").count();
    assert_eq!(count, 1, "should not duplicate entries");
}

#[test]
fn add_skill_gitignore_preserves_existing_content() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(content.contains("node_modules/"));
    assert!(content.contains(".agents/skills/brainstorming"));
}

#[test]
fn remove_skill_gitignore_entries_removes_all() {
    let project = tempfile::tempdir().unwrap();
    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();
    add_skill_entries(project.path(), "writing-plans", &[".claude/skills"]).unwrap();

    remove_skill_entries(project.path(), "brainstorming").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(!content.contains("brainstorming"));
    assert!(content.contains("writing-plans"));
}

#[test]
fn remove_skill_gitignore_entries_noop_if_not_present() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

    // Should not error
    remove_skill_entries(project.path(), "brainstorming").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert_eq!(content, "node_modules/\n");
}

#[test]
fn remove_skill_gitignore_cleans_empty_managed_section() {
    let project = tempfile::tempdir().unwrap();
    add_skill_entries(project.path(), "brainstorming", &[".claude/skills"]).unwrap();

    remove_skill_entries(project.path(), "brainstorming").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    // Should not leave behind a dangling "# Managed by ion" with no entries
    assert!(!content.contains("# Managed by ion"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill skill_gitignore`
Expected: FAIL — functions don't exist yet

**Step 3: Implement the functions**

Add these functions to `crates/ion-skill/src/gitignore.rs`:

```rust
/// Add per-skill gitignore entries for a remotely installed skill.
/// Creates entries for `.agents/skills/<name>` and `<target>/<name>` for each target.
/// Idempotent — won't duplicate existing entries.
pub fn add_skill_entries(project_dir: &Path, skill_name: &str, target_paths: &[&str]) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    let mut entries_to_add = vec![format!(".agents/skills/{skill_name}")];
    for target in target_paths {
        entries_to_add.push(format!("{target}/{skill_name}"));
    }

    // Filter out entries that already exist
    let existing_lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let new_entries: Vec<&String> = entries_to_add
        .iter()
        .filter(|e| !existing_lines.contains(&e.as_str()))
        .collect();

    if new_entries.is_empty() {
        return Ok(());
    }

    // Ensure trailing newline
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add managed section header if not present
    if !content.contains("# Managed by ion") {
        content.push_str("\n# Managed by ion\n");
    }

    for entry in new_entries {
        content.push_str(entry);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, &content).map_err(Error::Io)?;
    Ok(())
}

/// Remove all gitignore entries for a specific skill.
/// Removes any line containing `.agents/skills/<name>` or `<anything>/<name>` under the managed section.
/// Cleans up the "# Managed by ion" header if no managed entries remain.
pub fn remove_skill_entries(project_dir: &Path, skill_name: &str) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let content = match std::fs::read_to_string(&gitignore_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::Io(e)),
    };

    let skill_suffix = format!("/{skill_name}");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.ends_with(&skill_suffix)
        })
        .collect();

    // Clean up empty managed section
    let mut result: Vec<&str> = Vec::new();
    for (i, line) in filtered.iter().enumerate() {
        if line.trim() == "# Managed by ion" {
            // Check if there are any non-empty lines after this before the next section/end
            let has_entries = filtered[i + 1..]
                .iter()
                .take_while(|l| !l.starts_with('#') || l.trim().is_empty())
                .any(|l| !l.trim().is_empty());
            if !has_entries {
                // Skip this header and any trailing blank line before it
                while result.last().is_some_and(|l: &&str| l.trim().is_empty()) {
                    result.pop();
                }
                continue;
            }
        }
        result.push(line);
    }

    let mut output = result.join("\n");
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    std::fs::write(&gitignore_path, &output).map_err(Error::Io)?;
    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill gitignore`
Expected: All pass (old and new tests)

**Step 5: Commit**

```bash
git add crates/ion-skill/src/gitignore.rs
git commit -m "feat: add per-skill gitignore add/remove functions"
```

---

### Task 3: Change installer deploy to symlink instead of copy

**Files:**
- Modify: `crates/ion-skill/src/installer.rs:145-156`

**Step 1: Update the `deploy` method**

In `crates/ion-skill/src/installer.rs`, change the `deploy` method to symlink instead of copy for the `.agents/skills/<name>` entry:

```rust
fn deploy(&self, name: &str, skill_dir: &Path) -> Result<()> {
    let agents_target = self.project_dir.join(".agents").join("skills").join(name);
    create_skill_symlink(skill_dir, &agents_target)?;

    let canonical = self.project_dir.join(".agents").join("skills").join(name);
    for target_path in self.options.targets.values() {
        let target_skill_dir = self.project_dir.join(target_path).join(name);
        create_skill_symlink(&canonical, &target_skill_dir)?;
    }

    Ok(())
}
```

This replaces `copy_skill_dir(skill_dir, &agents_target)` with `create_skill_symlink(skill_dir, &agents_target)`.

**Step 2: Update the `uninstall` method**

The `.agents/skills/<name>` is now a symlink, not a directory. Update the removal logic:

```rust
pub fn uninstall(&self, name: &str) -> Result<()> {
    let agents_dir = self.project_dir.join(".agents").join("skills").join(name);
    if agents_dir.is_symlink() {
        std::fs::remove_file(&agents_dir).map_err(Error::Io)?;
    } else if agents_dir.exists() {
        std::fs::remove_dir_all(&agents_dir).map_err(Error::Io)?;
    }

    for target_path in self.options.targets.values() {
        let target_dir = self.project_dir.join(target_path).join(name);
        if target_dir.is_symlink() {
            std::fs::remove_file(&target_dir).map_err(Error::Io)?;
        } else if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir).map_err(Error::Io)?;
        }
    }

    Ok(())
}
```

**Step 3: Update existing tests**

In the `install_creates_symlinks_for_targets` test, change the assertion for the canonical path — it should now be a symlink, not a real directory:

```rust
#[test]
fn install_creates_symlinks_for_targets() {
    let skill_src = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_src.path().join("SKILL.md"),
        "---\nname: sym-test\ndescription: Symlink test.\n---\n\nBody.\n",
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    let source = SkillSource {
        source_type: SourceType::Path,
        source: skill_src.path().display().to_string(),
        path: None,
        rev: None,
        version: None,
    };

    let mut targets = std::collections::BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());
    let options = ManifestOptions { targets };

    let installer = SkillInstaller::new(project.path(), &options);
    let _locked = installer.install("sym-test", &source).unwrap();

    // Canonical is now a symlink to the source
    let canonical = project.path().join(".agents/skills/sym-test");
    assert!(canonical.exists());
    assert!(canonical.is_symlink());

    // Target is a symlink
    let target = project.path().join(".claude/skills/sym-test");
    assert!(target.exists());
    assert!(target.is_symlink());

    // Symlink resolves to the right content
    assert!(target.join("SKILL.md").exists());
}
```

Update `install_local_skill` similarly — the `.agents/skills/local-test` path should still resolve, but it's now a symlink:

```rust
#[test]
fn install_local_skill() {
    let skill_src = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_src.path().join("SKILL.md"),
        "---\nname: local-test\ndescription: A local test skill.\n---\n\nInstructions here.\n",
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    let source = test_source(skill_src.path());
    let options = empty_options();

    let installer = SkillInstaller::new(project.path(), &options);
    let locked = installer.install("local-test", &source).unwrap();
    assert_eq!(locked.name, "local-test");

    let agents_path = project.path().join(".agents/skills/local-test");
    assert!(agents_path.is_symlink());
    assert!(agents_path.join("SKILL.md").exists());
}
```

**Step 4: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 5: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "feat: deploy skills as symlinks instead of copies"
```

---

### Task 4: Wire per-skill gitignore into `ion add`

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Update `finish_single_install` to add gitignore entries**

In `src/commands/add.rs`, update the `finish_single_install` function to call `add_skill_entries` for remote skills (non-path sources):

```rust
fn finish_single_install(
    ctx: &ProjectContext,
    _installer: &SkillInstaller,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
) -> anyhow::Result<()> {
    println!("  Installed to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    // Add per-skill gitignore entries for remote skills
    if source.source_type != ion_skill::source::SourceType::Path {
        let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
        ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
    }

    manifest_writer::add_skill(&ctx.manifest_path, name, source)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
```

**Step 2: Update `install_collection` similarly**

In the `install_collection` function, after the `println!("    Installed to .agents/skills/{name}/");` block, add gitignore entries:

```rust
        // Add per-skill gitignore entries for remote skills
        let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
        ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass

**Step 4: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat: add per-skill gitignore entries on ion add"
```

---

### Task 5: Update `ion install` to use per-skill gitignore

**Files:**
- Modify: `src/commands/install.rs`

**Step 1: Remove the blanket gitignore prompt**

In `src/commands/install.rs`, replace the entire block from line 56 (`// Check gitignore for managed directories`) to line 86 with per-skill gitignore logic:

```rust
    // Add per-skill gitignore entries for remote skills
    let target_paths: Vec<&str> = merged_options.targets.values().map(|s| s.as_str()).collect();
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        if source.source_type != ion_skill::source::SourceType::Path {
            ion_skill::gitignore::add_skill_entries(&ctx.project_dir, name, &target_paths)?;
        }
    }
```

You'll need to add the import at the top:

```rust
use ion_skill::source::SourceType;
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All pass. The integration test `install_from_manifest` should still pass since it uses local path skills which don't trigger gitignore.

**Step 3: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: replace blanket gitignore with per-skill entries in ion install"
```

---

### Task 6: Update `ion remove` to clean up gitignore

**Files:**
- Modify: `src/commands/remove.rs`

**Step 1: Add gitignore cleanup**

In `src/commands/remove.rs`, after the `manifest_writer::remove_skill` call, add:

```rust
    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
```

The full updated function:

```rust
pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let manifest = ctx.manifest()?;

    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Removing skill '{name}'...");

    SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(name)?;
    println!("  Removed from .agents/skills/{name}/");

    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.remove(name);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
    println!("  Updated .gitignore");

    println!("Done!");
    Ok(())
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All pass

**Step 3: Commit**

```bash
git add src/commands/remove.rs
git commit -m "feat: clean up per-skill gitignore entries on ion remove"
```

---

### Task 7: Add `ion link` command

**Files:**
- Create: `src/commands/link.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add the CLI wiring**

In `src/main.rs`, add the `Link` variant to the `Commands` enum (after `Remove`):

```rust
    /// Link a local skill into the project
    Link {
        /// Path to a local skill directory containing SKILL.md
        path: String,
    },
```

Add the match arm (after `Commands::Remove`):

```rust
        Commands::Link { path } => commands::link::run(&path),
```

In `src/commands/mod.rs`, add:

```rust
pub mod link;
```

**Step 2: Create `src/commands/link.rs`**

```rust
use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::manifest_writer;
use ion_skill::source::{SkillSource, SourceType};

use crate::context::ProjectContext;
use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

pub fn run(path_str: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let path = std::path::PathBuf::from(path_str);
    let abs_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    if !abs_path.join("SKILL.md").exists() {
        anyhow::bail!("No SKILL.md found at {}", abs_path.display());
    }

    let source = SkillSource {
        source_type: SourceType::Path,
        source: path_str.to_string(),
        path: None,
        rev: None,
        version: None,
    };

    let name = abs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed-skill")
        .to_string();

    println!("Linking skill '{name}' from {path_str}...");

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    let locked = match installer.install(&name, &source) {
        Ok(locked) => locked,
        Err(SkillError::ValidationWarning { report, .. }) => {
            print_validation_report(&name, &report);
            if !confirm_install_on_warnings()? {
                anyhow::bail!("Link cancelled due to validation warnings.");
            }
            installer.install_with_options(
                &name,
                &source,
                InstallValidationOptions {
                    skip_validation: false,
                    allow_warnings: true,
                },
            )?
        }
        Err(err) => return Err(err.into()),
    };

    println!("  Linked to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    // No gitignore entries — local skills are tracked
    manifest_writer::add_skill(&ctx.manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass (compiles and existing tests unaffected)

**Step 4: Write integration test**

Create `tests/link_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn link_help_is_exposed() {
    let output = ion_cmd().args(["link", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Link a local skill"));
}

#[test]
fn link_local_skill_creates_symlinks() {
    let project = tempfile::tempdir().unwrap();
    // Create ion.toml with a target
    std::fs::write(
        project.path().join("ion.toml"),
        "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    // Create a local skill
    let skill_dir = project.path().join("skills").join("my-local-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local-skill\ndescription: A test local skill.\n---\n\nBody.\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["link", "skills/my-local-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    // .agents/skills/my-local-skill should be a symlink
    let agents_skill = project.path().join(".agents/skills/my-local-skill");
    assert!(agents_skill.is_symlink(), ".agents entry should be a symlink");
    assert!(agents_skill.join("SKILL.md").exists(), "symlink should resolve");

    // .claude/skills/my-local-skill should be a symlink
    let claude_skill = project.path().join(".claude/skills/my-local-skill");
    assert!(claude_skill.is_symlink(), "target entry should be a symlink");

    // Should NOT add gitignore entries for local skills
    let gitignore = project.path().join(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(
            !content.contains("my-local-skill"),
            "local skill should not be in .gitignore"
        );
    }

    // Should update ion.toml
    let manifest = std::fs::read_to_string(project.path().join("ion.toml")).unwrap();
    assert!(manifest.contains("my-local-skill"));
}

#[test]
fn link_missing_skill_md_errors() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("ion.toml"), "[skills]\n").unwrap();

    let empty_dir = project.path().join("empty-skill");
    std::fs::create_dir(&empty_dir).unwrap();

    let output = ion_cmd()
        .args(["link", "empty-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No SKILL.md"));
}
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/commands/link.rs src/commands/mod.rs src/main.rs tests/link_integration.rs
git commit -m "feat: add ion link command for local skills"
```

---

### Task 8: Create global registry module

**Files:**
- Create: `crates/ion-skill/src/registry.rs`
- Modify: `crates/ion-skill/src/lib.rs`

**Step 1: Write the tests first**

Create `crates/ion-skill/src/registry.rs` with tests:

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoEntry {
    pub url: String,
    #[serde(default)]
    pub projects: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub repos: BTreeMap<String, RepoEntry>,
}

impl Registry {
    /// Returns the path to the global registry file.
    pub fn registry_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("ion").join("registry.toml"))
    }

    /// Load the global registry. Returns empty registry if file doesn't exist.
    pub fn load() -> Result<Self> {
        match Self::registry_path() {
            Some(path) => Self::load_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load registry from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        crate::load_toml_or_default(path)
    }

    /// Save registry to the default path.
    pub fn save(&self) -> Result<()> {
        match Self::registry_path() {
            Some(path) => self.save_to(&path),
            None => Ok(()),
        }
    }

    /// Save registry to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Manifest(format!("Failed to serialize registry: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }

    /// Register that a project uses a specific repo.
    pub fn register(&mut self, repo_hash: &str, url: &str, project_dir: &str) {
        let entry = self.repos.entry(repo_hash.to_string()).or_insert_with(|| RepoEntry {
            url: url.to_string(),
            projects: Vec::new(),
        });
        if !entry.projects.contains(&project_dir.to_string()) {
            entry.projects.push(project_dir.to_string());
            entry.projects.sort();
        }
    }

    /// Unregister a project from a specific repo.
    pub fn unregister(&mut self, repo_hash: &str, project_dir: &str) {
        if let Some(entry) = self.repos.get_mut(repo_hash) {
            entry.projects.retain(|p| p != project_dir);
        }
    }

    /// Remove repos with no remaining projects. Returns list of removed repo hashes.
    pub fn cleanup_stale(&mut self) -> Vec<(String, String)> {
        let mut removed = Vec::new();
        self.repos.retain(|hash, entry| {
            // Remove projects whose directories no longer exist
            entry.projects.retain(|p| Path::new(p).exists());
            if entry.projects.is_empty() {
                removed.push((hash.clone(), entry.url.clone()));
                false
            } else {
                true
            }
        });
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister() {
        let mut registry = Registry::default();

        registry.register("abc123", "https://github.com/org/repo.git", "/home/user/project");
        assert_eq!(registry.repos["abc123"].projects.len(), 1);

        registry.register("abc123", "https://github.com/org/repo.git", "/home/user/project");
        assert_eq!(registry.repos["abc123"].projects.len(), 1, "should be idempotent");

        registry.register("abc123", "https://github.com/org/repo.git", "/home/user/other");
        assert_eq!(registry.repos["abc123"].projects.len(), 2);

        registry.unregister("abc123", "/home/user/project");
        assert_eq!(registry.repos["abc123"].projects.len(), 1);
    }

    #[test]
    fn cleanup_removes_nonexistent_projects() {
        let mut registry = Registry::default();
        registry.register("abc123", "https://github.com/org/repo.git", "/nonexistent/path/1");
        registry.register("abc123", "https://github.com/org/repo.git", "/nonexistent/path/2");

        let removed = registry.cleanup_stale();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "abc123");
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn cleanup_keeps_existing_projects() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::default();
        registry.register("abc123", "https://github.com/org/repo.git", &dir.path().display().to_string());
        registry.register("abc123", "https://github.com/org/repo.git", "/nonexistent/path");

        let removed = registry.cleanup_stale();

        assert!(removed.is_empty());
        assert_eq!(registry.repos["abc123"].projects.len(), 1);
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.toml");

        let mut registry = Registry::default();
        registry.register("abc123", "https://github.com/org/repo.git", "/home/user/project");

        registry.save_to(&path).unwrap();

        let loaded = Registry::load_from(&path).unwrap();
        assert_eq!(loaded.repos.len(), 1);
        assert_eq!(loaded.repos["abc123"].url, "https://github.com/org/repo.git");
        assert_eq!(loaded.repos["abc123"].projects, vec!["/home/user/project"]);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let registry = Registry::load_from(Path::new("/nonexistent/registry.toml")).unwrap();
        assert!(registry.repos.is_empty());
    }
}
```

**Step 2: Add module to lib.rs**

In `crates/ion-skill/src/lib.rs`, add:

```rust
pub mod registry;
```

**Step 3: Run tests**

Run: `cargo test -p ion-skill registry`
Expected: All pass

**Step 4: Commit**

```bash
git add crates/ion-skill/src/registry.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add global registry for tracking repo usage across projects"
```

---

### Task 9: Wire registry into `ion add` and `ion install`

**Files:**
- Modify: `src/commands/add.rs`
- Modify: `src/commands/install.rs`

**Step 1: Update `finish_single_install` in `add.rs`**

After the gitignore block, add registry registration for remote skills:

```rust
    // Register in global registry for gc
    if source.source_type != ion_skill::source::SourceType::Path {
        if let Ok(url) = source.git_url() {
            let repo_hash = format!("{:x}", ion_skill::installer::hash_simple(&url));
            let project_dir = ctx.project_dir.display().to_string();
            if let Ok(mut registry) = ion_skill::registry::Registry::load() {
                registry.register(&repo_hash, &url, &project_dir);
                let _ = registry.save();
            }
        }
    }
```

Add the same block in `install_collection` after the per-skill gitignore entries (just once, outside the skill loop, since all skills share the same repo).

**Step 2: Update `install.rs`**

After the per-skill gitignore loop, add registry registration:

```rust
    // Register repos in global registry for gc
    for (_name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        if source.source_type != SourceType::Path {
            if let Ok(url) = source.git_url() {
                let repo_hash = format!("{:x}", ion_skill::installer::hash_simple(&url));
                let project_dir = ctx.project_dir.display().to_string();
                if let Ok(mut registry) = ion_skill::registry::Registry::load() {
                    registry.register(&repo_hash, &url, &project_dir);
                    let _ = registry.save();
                }
            }
        }
    }
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass

**Step 4: Commit**

```bash
git add src/commands/add.rs src/commands/install.rs
git commit -m "feat: register repos in global registry on add/install"
```

---

### Task 10: Add `ion gc` command

**Files:**
- Create: `src/commands/gc.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add CLI wiring**

In `src/main.rs`, add the `Gc` variant:

```rust
    /// Garbage collect stale skill repos
    Gc {
        /// Show what would be removed without deleting
        #[arg(long)]
        dry_run: bool,
    },
```

Add the match arm:

```rust
        Commands::Gc { dry_run } => commands::gc::run(dry_run),
```

In `src/commands/mod.rs`, add:

```rust
pub mod gc;
```

**Step 2: Create `src/commands/gc.rs`**

```rust
use ion_skill::installer::data_dir;
use ion_skill::registry::Registry;

pub fn run(dry_run: bool) -> anyhow::Result<()> {
    let mut registry = Registry::load()?;

    println!("Scanning for stale repos...");

    // Clean up projects that no longer exist or no longer reference the repo
    let removed = registry.cleanup_stale();

    if removed.is_empty() {
        println!("No stale repos found.");
        return Ok(());
    }

    let repo_base = data_dir();

    for (hash, url) in &removed {
        let repo_path = repo_base.join(hash);
        if dry_run {
            println!("  Would remove: {url} ({hash})");
            if repo_path.exists() {
                println!("    Path: {}", repo_path.display());
            }
        } else {
            println!("  Removing: {url} ({hash})");
            if repo_path.exists() {
                std::fs::remove_dir_all(&repo_path)?;
            }
        }
    }

    if dry_run {
        println!("\n{} repo(s) would be removed. Run without --dry-run to delete.", removed.len());
    } else {
        registry.save()?;
        println!("\nRemoved {} stale repo(s).", removed.len());
    }

    Ok(())
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All compile and pass

**Step 4: Write integration test**

Add to `tests/integration.rs` (or create `tests/gc_integration.rs`):

Create `tests/gc_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn gc_help_is_exposed() {
    let output = ion_cmd().args(["gc", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Garbage collect"));
}

#[test]
fn gc_dry_run_with_no_stale_repos() {
    let output = ion_cmd().args(["gc", "--dry-run"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
    // Should not error even with empty/no registry
}
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/commands/gc.rs src/commands/mod.rs src/main.rs tests/gc_integration.rs
git commit -m "feat: add ion gc command for stale repo cleanup"
```

---

### Task 11: Storage migration from cache to data dir

**Files:**
- Modify: `crates/ion-skill/src/installer.rs`

**Step 1: Add migration function**

Add a public function in `crates/ion-skill/src/installer.rs`:

```rust
/// Migrate repos from the old cache dir to the new data dir if needed.
/// This is a one-time migration that runs on first use.
pub fn migrate_cache_to_data() -> Result<()> {
    let old_cache = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("repos");

    let new_data = data_dir();

    // Only migrate if old exists and new doesn't
    if !old_cache.exists() || new_data.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(new_data.parent().unwrap_or(&new_data)).map_err(Error::Io)?;

    // Move each repo directory
    for entry in std::fs::read_dir(&old_cache).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        if entry.path().is_dir() {
            let dest = new_data.join(entry.file_name());
            std::fs::rename(entry.path(), &dest).map_err(Error::Io)?;
        }
    }

    // Clean up old directory
    let _ = std::fs::remove_dir_all(&old_cache);

    Ok(())
}
```

**Step 2: Call migration from `fetch_skill_base`**

At the start of `fetch_skill_base`, add:

```rust
fn fetch_skill_base(source: &SkillSource) -> Result<PathBuf> {
    // One-time migration from old cache dir
    let _ = migrate_cache_to_data();

    match source.source_type {
        // ... rest unchanged
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass

**Step 4: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "feat: auto-migrate repos from cache dir to data dir"
```
