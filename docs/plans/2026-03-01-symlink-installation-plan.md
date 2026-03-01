# Symlink-based Skill Installation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace file copying with directory symlinks for secondary agent targets, generalize target configuration, and add gitignore prompting.

**Architecture:** `.agents/skills/<name>/` stays as the canonical copy (real files from cache). Secondary targets (`.claude/skills/`, `.cursor/skills/`, etc.) become relative directory symlinks. Target configuration moves from a boolean `install-to-claude` to a named `[options.targets]` map. After install, ion prompts to add managed directories to `.gitignore`.

**Tech Stack:** Rust, toml/toml_edit for config, std::os::unix::fs::symlink for symlinks, std::fs for file ops.

---

### Task 1: Replace `ManifestOptions` with targets map

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs:26-31` (ManifestOptions struct)
- Test: `crates/ion-skill/src/manifest.rs` (existing tests in same file)

**Step 1: Write the failing test**

Add this test to the `mod tests` block in `crates/ion-skill/src/manifest.rs`:

```rust
#[test]
fn parse_targets_options() {
    let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert_eq!(manifest.options.targets.len(), 2);
    assert_eq!(manifest.options.targets["claude"], ".claude/skills");
    assert_eq!(manifest.options.targets["cursor"], ".cursor/skills");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ion-skill parse_targets_options`
Expected: FAIL — `ManifestOptions` has no `targets` field.

**Step 3: Replace ManifestOptions**

In `crates/ion-skill/src/manifest.rs`, replace:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestOptions {
    #[serde(default)]
    pub install_to_claude: bool,
}
```

With:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestOptions {
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
}
```

**Step 4: Fix all compile errors from the removal of `install_to_claude`**

Every file that references `install_to_claude` or `ManifestOptions { install_to_claude: ... }` must be updated. Grep the codebase for `install_to_claude` to find them all. The main locations are:

- `crates/ion-skill/src/installer.rs` — `install_skill()` and `uninstall_skill()` use `options.install_to_claude`; update in the next task.
- `crates/ion-skill/src/manifest.rs` — tests that construct `ManifestOptions` or assert on `install_to_claude`.
- `tests/integration.rs` — integration tests may reference this option.

For now, just fix the tests in `manifest.rs`. Update the `parse_options` test to test the new format, and update `parse_empty_manifest` to check `targets.is_empty()`:

```rust
#[test]
fn parse_options() {
    let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert_eq!(manifest.options.targets["claude"], ".claude/skills");
}

#[test]
fn parse_empty_manifest() {
    let manifest = Manifest::parse("[skills]\n").unwrap();
    assert!(manifest.skills.is_empty());
    assert!(manifest.options.targets.is_empty());
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p ion-skill -- parse_targets_options parse_options parse_empty_manifest`
Expected: All PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "refactor: replace install_to_claude with targets map in ManifestOptions"
```

---

### Task 2: Add symlink creation to the installer

**Files:**
- Modify: `crates/ion-skill/src/installer.rs:18-81` (install_skill function)
- Modify: `crates/ion-skill/src/installer.rs:155-167` (uninstall_skill function)
- Test: `crates/ion-skill/src/installer.rs` (existing tests in same file)

**Step 1: Write the failing test for symlink creation**

Add to `mod tests` in `crates/ion-skill/src/installer.rs`:

```rust
#[test]
fn install_creates_symlinks_for_targets() {
    let skill_src = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_src.path().join("SKILL.md"),
        "---\nname: sym-test\ndescription: Symlink test.\n---\n\nBody.\n",
    ).unwrap();

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

    let _locked = install_skill(project.path(), "sym-test", &source, &options).unwrap();

    // Canonical copy is a real directory
    let canonical = project.path().join(".agents/skills/sym-test");
    assert!(canonical.exists());
    assert!(canonical.is_dir());
    assert!(!canonical.is_symlink());

    // Target is a symlink
    let target = project.path().join(".claude/skills/sym-test");
    assert!(target.exists());
    assert!(target.is_symlink());

    // Symlink resolves to the right content
    assert!(target.join("SKILL.md").exists());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ion-skill install_creates_symlinks_for_targets`
Expected: FAIL — `ManifestOptions` has no `targets` field yet in installer tests (compile error) or the symlink assertion fails.

**Step 3: Update `install_skill()` to create symlinks**

In `crates/ion-skill/src/installer.rs`, replace the section that copies to `.claude/skills/`:

```rust
// OLD:
// if options.install_to_claude {
//     let claude_target = project_dir.join(".claude").join("skills").join(name);
//     copy_skill_dir(&skill_dir, &claude_target)?;
// }

// NEW:
// Create symlinks for each configured target
let canonical = project_dir.join(".agents").join("skills").join(name);
for (_target_name, target_path) in &options.targets {
    let target_skill_dir = project_dir.join(target_path).join(name);
    create_skill_symlink(&canonical, &target_skill_dir)?;
}
```

Add the `create_skill_symlink` function:

```rust
/// Create a relative symlink from `link` pointing to `original`.
fn create_skill_symlink(original: &Path, link: &Path) -> Result<()> {
    // Remove existing file/dir/symlink at the link location
    if link.is_symlink() {
        std::fs::remove_file(link).map_err(Error::Io)?;
    } else if link.exists() {
        std::fs::remove_dir_all(link).map_err(Error::Io)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    // Compute relative path from link's parent to the original
    let link_parent = link.parent().unwrap();
    let relative = pathdiff::diff_paths(original, link_parent)
        .ok_or_else(|| Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Cannot compute relative path from {} to {}", link_parent.display(), original.display()),
        )))?;

    #[cfg(unix)]
    std::os::unix::fs::symlink(&relative, link).map_err(Error::Io)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&relative, link).map_err(Error::Io)?;

    Ok(())
}
```

Note: Add `pathdiff` as a dependency to `crates/ion-skill/Cargo.toml`:

```toml
[dependencies]
pathdiff = "0.2"
```

**Step 4: Update `uninstall_skill()` to remove symlinks from targets**

Replace the current `uninstall_skill` function:

```rust
pub fn uninstall_skill(project_dir: &Path, name: &str, options: &ManifestOptions) -> Result<()> {
    // Remove canonical copy
    let agents_dir = project_dir.join(".agents").join("skills").join(name);
    if agents_dir.exists() {
        std::fs::remove_dir_all(&agents_dir).map_err(Error::Io)?;
    }

    // Remove symlinks from all targets
    for (_target_name, target_path) in &options.targets {
        let target_dir = project_dir.join(target_path).join(name);
        if target_dir.is_symlink() {
            std::fs::remove_file(&target_dir).map_err(Error::Io)?;
        } else if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir).map_err(Error::Io)?;
        }
    }

    Ok(())
}
```

**Step 5: Fix existing tests in `installer.rs`**

Update the `uninstall_removes_dirs` test to use the new `ManifestOptions` format:

```rust
#[test]
fn uninstall_removes_dirs() {
    let project = tempfile::tempdir().unwrap();
    let agents = project.path().join(".agents").join("skills").join("test");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::write(agents.join("SKILL.md"), "x").unwrap();

    // Create a symlink target
    let claude = project.path().join(".claude").join("skills");
    std::fs::create_dir_all(&claude).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        std::path::Path::new("../../../.agents/skills/test"),
        claude.join("test"),
    ).unwrap();

    let mut targets = std::collections::BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());
    let options = ManifestOptions { targets };
    uninstall_skill(project.path(), "test", &options).unwrap();

    assert!(!agents.exists());
    assert!(!claude.join("test").exists());
}
```

Update the `install_local_skill` test to use `ManifestOptions { targets: BTreeMap::new() }`:

```rust
let options = ManifestOptions { targets: std::collections::BTreeMap::new() };
```

**Step 6: Run all installer tests**

Run: `cargo test -p ion-skill -- installer`
Expected: All PASS.

**Step 7: Commit**

```bash
git add crates/ion-skill/Cargo.toml crates/ion-skill/src/installer.rs
git commit -m "feat: create symlinks for secondary targets instead of copying"
```

---

### Task 3: Add gitignore checking and prompting

**Files:**
- Create: `crates/ion-skill/src/gitignore.rs`
- Modify: `crates/ion-skill/src/lib.rs` (add module)
- Test: `crates/ion-skill/src/gitignore.rs` (inline tests)

**Step 1: Write the failing test**

Create `crates/ion-skill/src/gitignore.rs` with tests first:

```rust
use std::path::Path;

use crate::{Error, Result};

/// Check which directories from the given list are missing from .gitignore.
/// Returns the list of directories that are NOT in .gitignore.
pub fn find_missing_gitignore_entries(project_dir: &Path, dirs: &[&str]) -> Result<Vec<String>> {
    todo!()
}

/// Append entries to .gitignore, creating it if it doesn't exist.
pub fn append_to_gitignore(project_dir: &Path, entries: &[&str]) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_missing_entries() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), ".agents/\n").unwrap();

        let missing = find_missing_gitignore_entries(
            project.path(),
            &[".agents/", ".claude/"],
        ).unwrap();

        assert_eq!(missing, vec![".claude/"]);
    }

    #[test]
    fn no_gitignore_means_all_missing() {
        let project = tempfile::tempdir().unwrap();

        let missing = find_missing_gitignore_entries(
            project.path(),
            &[".agents/", ".claude/"],
        ).unwrap();

        assert_eq!(missing, vec![".agents/", ".claude/"]);
    }

    #[test]
    fn append_creates_gitignore() {
        let project = tempfile::tempdir().unwrap();

        append_to_gitignore(project.path(), &[".agents/", ".claude/"]).unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains(".agents/"));
        assert!(content.contains(".claude/"));
    }

    #[test]
    fn append_adds_to_existing_gitignore() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

        append_to_gitignore(project.path(), &[".agents/"]).unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".agents/"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill gitignore`
Expected: FAIL — `todo!()` panics.

**Step 3: Implement the functions**

Replace the `todo!()` bodies:

```rust
pub fn find_missing_gitignore_entries(project_dir: &Path, dirs: &[&str]) -> Result<Vec<String>> {
    let gitignore_path = project_dir.join(".gitignore");
    let content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    let existing: Vec<&str> = content.lines().map(|l| l.trim()).collect();

    Ok(dirs
        .iter()
        .filter(|d| !existing.contains(d))
        .map(|d| d.to_string())
        .collect())
}

pub fn append_to_gitignore(project_dir: &Path, entries: &[&str]) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    // Ensure there's a newline before our additions
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add a section comment
    content.push_str("\n# Managed by ion\n");
    for entry in entries {
        content.push_str(entry);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, &content).map_err(Error::Io)?;
    Ok(())
}
```

**Step 4: Register the module**

In `crates/ion-skill/src/lib.rs`, add:

```rust
pub mod gitignore;
```

**Step 5: Run tests**

Run: `cargo test -p ion-skill gitignore`
Expected: All PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/gitignore.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add gitignore checking and append utilities"
```

---

### Task 4: Integrate gitignore prompting into the install command

**Files:**
- Modify: `src/commands/install.rs`

**Step 1: Update the install command to prompt about gitignore**

Add this after the install loop in `src/commands/install.rs`:

```rust
use ion_skill::gitignore;
use std::io::{self, Write};

// ... after the install loop and lockfile write ...

// Collect all managed directories
let mut managed_dirs = vec![".agents/".to_string()];
for (_name, path) in &manifest.options.targets {
    // Extract the top-level directory (e.g., ".claude/skills" -> ".claude/")
    let top_level = path.split('/').next().unwrap_or(path);
    let entry = format!("{top_level}/");
    if !managed_dirs.contains(&entry) {
        managed_dirs.push(entry);
    }
}

let dir_refs: Vec<&str> = managed_dirs.iter().map(|s| s.as_str()).collect();
let missing = gitignore::find_missing_gitignore_entries(&project_dir, &dir_refs)?;

if !missing.is_empty() {
    println!("\nThese directories are not in .gitignore:");
    for dir in &missing {
        println!("  {dir}");
    }
    print!("\nAdd them? [y/n] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    if answer.trim().eq_ignore_ascii_case("y") {
        let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        gitignore::append_to_gitignore(&project_dir, &refs)?;
        println!("Updated .gitignore");
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: prompt to add managed directories to .gitignore after install"
```

---

### Task 5: Reject old `install-to-claude` config with migration guidance

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs`

**Step 1: Write the failing test**

Add to `mod tests` in `crates/ion-skill/src/manifest.rs`:

```rust
#[test]
fn rejects_old_install_to_claude_option() {
    let toml_str = "[skills]\n\n[options]\ninstall-to-claude = true\n";
    let result = Manifest::parse(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("install-to-claude"), "Error should mention the old option: {err_msg}");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ion-skill rejects_old_install_to_claude_option`
Expected: FAIL — serde currently ignores unknown fields (the parse succeeds instead of erroring).

**Step 3: Add validation in `Manifest::parse`**

Two options here. The simplest: use `serde(deny_unknown_fields)` on `ManifestOptions`. But that would reject all unknown fields, not just `install-to-claude`. A more targeted approach: parse first, then check the raw TOML for the old key.

Update `Manifest::parse`:

```rust
pub fn parse(content: &str) -> Result<Self> {
    // Check for deprecated options before parsing
    let raw: toml::Value = toml::from_str(content).map_err(Error::TomlParse)?;
    if let Some(options) = raw.get("options") {
        if options.get("install-to-claude").is_some() {
            return Err(Error::Manifest(
                "'install-to-claude' is no longer supported. Use [options.targets] instead:\n\n\
                 [options.targets]\n\
                 claude = \".claude/skills\"\n".to_string()
            ));
        }
    }

    toml::from_str(content).map_err(Error::TomlParse)
}
```

**Step 4: Run test**

Run: `cargo test -p ion-skill rejects_old_install_to_claude_option`
Expected: PASS.

**Step 5: Run all manifest tests**

Run: `cargo test -p ion-skill -- manifest`
Expected: All PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "feat: reject old install-to-claude option with migration guidance"
```

---

### Task 6: Update integration tests

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Update `install_from_manifest` test to use `[options.targets]`**

Update the test to use the new config format and verify symlinks are created:

```rust
#[test]
fn install_from_manifest() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("manifest-skill");
    std::fs::create_dir(&skill_path).unwrap();

    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: manifest-skill\ndescription: Manifest test.\n---\n\nBody.\n",
    )
    .unwrap();

    // Use new [options.targets] format
    std::fs::write(
        project.path().join("ion.toml"),
        format!(
            "[skills]\nmanifest-skill = {{ type = \"path\", source = \"{}\" }}\n\n[options.targets]\nclaude = \".claude/skills\"\n",
            skill_path.display()
        ),
    )
    .unwrap();

    let output = ion_cmd()
        .args(["install"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "install failed: {stderr}");

    // Canonical copy exists as real directory
    assert!(project.path().join(".agents/skills/manifest-skill/SKILL.md").exists());

    // Target is a symlink
    let target = project.path().join(".claude/skills/manifest-skill");
    assert!(target.is_symlink());
    assert!(target.join("SKILL.md").exists());
}
```

**Step 2: Run integration tests**

Run: `cargo test --test integration`
Expected: All PASS.

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: update integration tests for symlink-based installation"
```

---

### Task 7: Run full test suite and fix any remaining issues

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All PASS.

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 3: Fix any remaining issues**

If any tests fail or clippy reports issues, fix them.

**Step 4: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "fix: address remaining issues from symlink migration"
```

---

## Execution notes

- **Task order matters:** Task 1 must be completed before Task 2 (struct change needed first). Tasks 3 and 5 are independent of each other but both depend on Task 1. Task 4 depends on Task 3. Task 6 depends on Tasks 2 and 4. Task 7 is always last.
- **`pathdiff` crate:** Task 2 adds this dependency — it computes relative paths between two absolute paths. Verify it's available on crates.io.
- **Windows support:** The symlink function uses `#[cfg(unix)]` / `#[cfg(windows)]`. On Windows, directory symlinks require elevated privileges. This is acceptable for now.
- **The `stdin` prompt in Task 4:** The integration tests pipe stdin, so the gitignore prompt won't block in tests (stdin will return empty/EOF, which means the prompt defaults to "no"). If this causes issues, wrap the prompt behind an `atty::is(atty::Stream::Stdin)` check.
