# Track 1: Constants & Small Fixes — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish single source of truth for defaults, fix hardcoded path bugs, fix silent errors, and extract `unregister_from_registry()`.

**Architecture:** Purely additive changes — new constant, new methods, fixing call sites. No structural changes to types or modules. Each task is independent and can be committed separately.

**Tech Stack:** Rust, toml/serde, existing `ion-skill` and `ion` crates.

**Spec:** `docs/superpowers/specs/2026-03-27-codebase-refactor-design.md` — Track 1.

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `crates/ion-skill/src/manifest.rs` | Add `DEFAULT_SKILLS_DIR` constant and `skills_dir_or_default()` method |
| Modify | `crates/ion-skill/src/source.rs` | Add `is_local_path()` method |
| Modify | `crates/ion-skill/src/installer.rs` | Fix `skill_dir()` to use options instead of hardcoded path |
| Modify | `crates/ion-skill/src/update/binary.rs` | Use `installer.skill_dir()` instead of hardcoded path |
| Modify | `src/commands/new.rs` | Use `DEFAULT_SKILLS_DIR` |
| Modify | `src/commands/eject.rs` | Use `skills_dir_or_default()` |
| Modify | `src/commands/install.rs` | Use `skills_dir_or_default()` |
| Modify | `src/tui/app.rs` | Use `DEFAULT_SKILLS_DIR` |
| Modify | `src/commands/info.rs` | Fix hardcoded path using `skills_dir_or_default()` |
| Modify | `src/commands/list.rs` | Fix hardcoded path using `skills_dir_or_default()` |
| Modify | `src/commands/update.rs` | Warn on `entry.resolve()` errors instead of silently dropping |
| Modify | `src/commands/install_shared.rs` | Add `unregister_from_registry()` |
| Modify | `src/commands/remove.rs` | Use `unregister_from_registry()` |

---

### Task 1: Add `DEFAULT_SKILLS_DIR` constant and `skills_dir_or_default()` method

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs`

- [ ] **Step 1: Write the test for `skills_dir_or_default()`**

In `crates/ion-skill/src/manifest.rs`, find the existing `#[cfg(test)] mod tests` block and add:

```rust
#[test]
fn skills_dir_or_default_uses_default() {
    let opts = ManifestOptions {
        targets: std::collections::BTreeMap::new(),
        skills_dir: None,
    };
    assert_eq!(opts.skills_dir_or_default(), ".agents/skills");
}

#[test]
fn skills_dir_or_default_uses_custom() {
    let opts = ManifestOptions {
        targets: std::collections::BTreeMap::new(),
        skills_dir: Some("custom/skills".to_string()),
    };
    assert_eq!(opts.skills_dir_or_default(), "custom/skills");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -E 'test(skills_dir_or_default)' -p ion-skill`
Expected: FAIL — `skills_dir_or_default` method does not exist yet.

- [ ] **Step 3: Add the constant and method**

In `crates/ion-skill/src/manifest.rs`, add near the top (after imports):

```rust
/// Default directory where skills are installed within a project.
pub const DEFAULT_SKILLS_DIR: &str = ".agents/skills";
```

Then add to `impl ManifestOptions` (find the existing impl block around line 106):

```rust
/// Returns the configured skills directory, or the default `.agents/skills`.
pub fn skills_dir_or_default(&self) -> &str {
    self.skills_dir.as_deref().unwrap_or(DEFAULT_SKILLS_DIR)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run -E 'test(skills_dir_or_default)' -p ion-skill`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "feat: add DEFAULT_SKILLS_DIR constant and skills_dir_or_default() method"
```

---

### Task 2: Fix `SkillInstaller::skill_dir()` to use options

**Files:**
- Modify: `crates/ion-skill/src/installer.rs:69-71`

- [ ] **Step 1: Verify current hardcoded code**

Read `crates/ion-skill/src/installer.rs` line 69-71. Confirm it says:
```rust
pub fn skill_dir(&self, name: &str) -> PathBuf {
    self.project_dir.join(".agents").join("skills").join(name)
}
```

- [ ] **Step 2: Replace with options-based path**

Change `skill_dir()` to:
```rust
pub fn skill_dir(&self, name: &str) -> PathBuf {
    self.project_dir.join(self.options.skills_dir_or_default()).join(name)
}
```

- [ ] **Step 3: Run tests**

Run: `cargo nextest run`
Expected: All tests pass. Existing tests use `ManifestOptions { skills_dir: None, .. }` which defaults to `.agents/skills`, so behavior is unchanged.

- [ ] **Step 4: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "fix: use skills_dir_or_default() in SkillInstaller::skill_dir()"
```

---

### Task 3: Fix `BinaryUpdater` hardcoded path

**Files:**
- Modify: `crates/ion-skill/src/update/binary.rs:53-57`

- [ ] **Step 1: Read the current code**

Read `crates/ion-skill/src/update/binary.rs` around lines 50-60. Find the line that constructs:
```rust
let skill_dir = installer.project_dir().join(".agents").join("skills").join(&skill.name);
```

- [ ] **Step 2: Replace with `installer.skill_dir()`**

Change to:
```rust
let skill_dir = installer.skill_dir(&skill.name);
```

- [ ] **Step 3: Run tests**

Run: `cargo nextest run`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/ion-skill/src/update/binary.rs
git commit -m "fix: use installer.skill_dir() in BinaryUpdater instead of hardcoded path"
```

---

### Task 4: Replace hardcoded `.agents/skills` in CLI commands

**Files:**
- Modify: `src/commands/new.rs:8`
- Modify: `src/commands/eject.rs:30`
- Modify: `src/commands/install.rs:76`
- Modify: `src/tui/app.rs:152`

- [ ] **Step 1: Fix `new.rs`**

In `src/commands/new.rs`, find line 8:
```rust
const DEFAULT_SKILLS_DIR: &str = ".agents/skills";
```
Remove this local constant. Replace all references to `DEFAULT_SKILLS_DIR` in this file with `ion_skill::manifest::DEFAULT_SKILLS_DIR`. Add the import at the top if not already present.

- [ ] **Step 2: Fix `eject.rs`**

In `src/commands/eject.rs`, find line ~30 where it says `.unwrap_or(".agents/skills")`. Replace with:
```rust
.unwrap_or(ion_skill::manifest::DEFAULT_SKILLS_DIR)
```

If the file uses `merged_options`, prefer `merged_options.skills_dir_or_default()` instead.

- [ ] **Step 3: Fix `install.rs`**

In `src/commands/install.rs`, find line ~76 where it says `.unwrap_or(".agents/skills")`. Replace with:
```rust
merged_options.skills_dir_or_default()
```

- [ ] **Step 4: Fix `tui/app.rs`**

In `src/tui/app.rs`, find line ~152 referencing `".agents/skills"`. Replace with `ion_skill::manifest::DEFAULT_SKILLS_DIR`.

- [ ] **Step 5: Run clippy and tests**

Run: `cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run`
Expected: No warnings, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/commands/new.rs src/commands/eject.rs src/commands/install.rs src/tui/app.rs
git commit -m "refactor: replace hardcoded .agents/skills with DEFAULT_SKILLS_DIR"
```

---

### Task 5: Add `is_local_path()` method to `SkillSource`

**Files:**
- Modify: `crates/ion-skill/src/source.rs`

- [ ] **Step 1: Write the test**

In `crates/ion-skill/src/source.rs`, find the test module and add:

```rust
#[test]
fn is_local_path_for_path_source() {
    let s = SkillSource::infer("./my-skill").unwrap();
    assert!(s.is_local_path());
}

#[test]
fn is_local_path_for_absolute_path() {
    let s = SkillSource::infer("/home/user/skill").unwrap();
    assert!(s.is_local_path());
}

#[test]
fn is_local_path_false_for_github() {
    let s = SkillSource::infer("org/repo").unwrap();
    assert!(!s.is_local_path());
}

#[test]
fn is_local_path_false_for_url() {
    let s = SkillSource::infer("https://github.com/org/repo.git").unwrap();
    assert!(!s.is_local_path());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(is_local_path)' -p ion-skill`
Expected: FAIL — `is_local_path` method does not exist.

- [ ] **Step 3: Add the method**

In `crates/ion-skill/src/source.rs`, add to `impl SkillSource`:

```rust
/// Returns true if the source string points to a local filesystem path.
pub fn is_local_path(&self) -> bool {
    self.source.starts_with('/')
        || self.source.starts_with("./")
        || self.source.starts_with("../")
}
```

- [ ] **Step 4: Run tests**

Run: `cargo nextest run -E 'test(is_local_path)' -p ion-skill`
Expected: PASS

- [ ] **Step 5: Replace inline checks in `add.rs` and `installer.rs`**

In `src/commands/add.rs`, find lines ~41-43:
```rust
let is_local_path = source.source.starts_with('/')
    || source.source.starts_with("./")
    || source.source.starts_with("../");
```
Replace with:
```rust
let is_local_path = source.is_local_path();
```

In `crates/ion-skill/src/installer.rs`, find lines ~229-231 (inside `install_binary()`):
```rust
let is_local_path = source.source.starts_with('/')
    || source.source.starts_with("./")
    || source.source.starts_with("../");
```
Replace with:
```rust
let is_local_path = source.is_local_path();
```

- [ ] **Step 6: Run full tests**

Run: `cargo nextest run`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/ion-skill/src/source.rs src/commands/add.rs crates/ion-skill/src/installer.rs
git commit -m "refactor: add SkillSource::is_local_path() and replace inline checks"
```

---

### Task 6: Fix hardcoded paths in `info.rs` and `list.rs`

**Files:**
- Modify: `src/commands/info.rs:47-53`
- Modify: `src/commands/list.rs:39-44, 86-91`

- [ ] **Step 1: Fix `info.rs`**

Read `src/commands/info.rs`. In `show_info_from_installed()` (around line 47), find:
```rust
let skill_md = ctx
    .project_dir
    .join(".agents")
    .join("skills")
    .join(name)
    .join("SKILL.md");
```

This function doesn't currently have access to `merged_options`. Add it:
1. Load the manifest: `let manifest = ctx.manifest()?;`
2. Get merged options: `let merged_options = ctx.merged_options(&manifest);`
3. Replace the hardcoded path:
```rust
let skill_md = ctx.project_dir
    .join(merged_options.skills_dir_or_default())
    .join(name)
    .join("SKILL.md");
```

Note: `show_info_from_installed` is called from `run()` which already has the manifest available. If the function signature allows, pass `merged_options` as a parameter instead of reloading.

- [ ] **Step 2: Fix `list.rs` — JSON mode**

In `src/commands/list.rs`, find the JSON block (around line 39-44):
```rust
let installed = ctx
    .project_dir
    .join(".agents")
    .join("skills")
    .join(name)
    .exists();
```

The function already has `manifest` and `ctx`. Add `let merged_options = ctx.merged_options(&manifest);` before the loop and replace:
```rust
let installed = ctx.project_dir
    .join(merged_options.skills_dir_or_default())
    .join(name)
    .exists();
```

- [ ] **Step 3: Fix `list.rs` — human mode**

Find the similar block around lines 86-91 and apply the same fix. The `merged_options` variable should be computed once before both loops.

- [ ] **Step 4: Run clippy and tests**

Run: `cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run`
Expected: No warnings, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/info.rs src/commands/list.rs
git commit -m "fix: use skills_dir_or_default() in info and list commands"
```

---

### Task 7: Fix silent `entry.resolve()` errors in `update.rs` and `list.rs`

**Files:**
- Modify: `src/commands/update.rs:20-28`
- Modify: `src/commands/list.rs:25-27`

- [ ] **Step 1: Fix `update.rs`**

In `src/commands/update.rs`, find the `filter_map` block (around lines 20-28):
```rust
.filter_map(|(skill_name, entry)| {
    let source = entry.resolve().ok()?;
    Some((skill_name.clone(), source))
})
```

Replace with:
```rust
.filter_map(|(skill_name, entry)| {
    match entry.resolve() {
        Ok(source) => Some((skill_name.clone(), source)),
        Err(e) => {
            eprintln!("Warning: skipping '{}': {}", skill_name, e);
            None
        }
    }
})
```

- [ ] **Step 2: Fix `list.rs` JSON mode**

In `src/commands/list.rs`, find the JSON block's `filter_map` (around line 25-27):
```rust
.filter_map(|(name, entry)| {
    let source = entry.resolve().ok()?;
```

Replace with the same pattern — warn and skip:
```rust
.filter_map(|(name, entry)| {
    let source = match entry.resolve() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: skipping '{}': {}", name, e);
            return None;
        }
    };
```

- [ ] **Step 3: Run tests**

Run: `cargo nextest run`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/update.rs src/commands/list.rs
git commit -m "fix: warn instead of silently dropping entry.resolve() errors"
```

---

### Task 8: Add `unregister_from_registry()` and use it in `remove.rs`

**Files:**
- Modify: `src/commands/install_shared.rs`
- Modify: `src/commands/remove.rs:117-127`

- [ ] **Step 1: Add `unregister_from_registry()` to `install_shared.rs`**

In `src/commands/install_shared.rs`, add after the existing `register_in_registry()` function:

```rust
/// Unregister a git-based skill source from the global registry.
pub fn unregister_from_registry(
    source: &SkillSource,
    project_dir: &std::path::Path,
) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git) {
        if let Ok(url) = source.git_url() {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.unregister(&repo_hash, &project_str);
            registry.save()?;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Replace inline code in `remove.rs`**

In `src/commands/remove.rs`, find lines ~117-127:
```rust
if let Ok(ref source) = entry_source
    && matches!(source.source_type, SourceType::Github | SourceType::Git)
    && let Ok(url) = source.git_url()
{
    let repo_hash = format!("{:x}", hash_simple(&url));
    let project_str = ctx.project_dir.display().to_string();
    let mut registry = Registry::load()?;
    registry.unregister(&repo_hash, &project_str);
    registry.save()?;
}
```

Replace with:
```rust
if let Ok(ref source) = entry_source {
    crate::commands::install_shared::unregister_from_registry(source, &ctx.project_dir)?;
}
```

Remove unused imports from `remove.rs` that were only used for the inline registry code: `hash_simple` and `Registry` (check if they're used elsewhere in the file first).

- [ ] **Step 3: Run clippy and tests**

Run: `cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run`
Expected: No warnings, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/install_shared.rs src/commands/remove.rs
git commit -m "refactor: extract unregister_from_registry() to install_shared"
```

---

### Task 9: Final verification

- [ ] **Step 1: Run full pre-commit checks**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
```

Expected: All three pass cleanly.

- [ ] **Step 2: Review the diff**

```bash
git log --oneline main..HEAD
```

Verify there are 8 commits (one per task above, except the final verification).
