# Update Non-Binary Skills Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend `ion update` to update git/GitHub skills (not just binaries), using an `Updater` trait with per-source-type implementations.

**Architecture:** New `ion-skill/src/update/` module with an `Updater` trait. `BinaryUpdater` extracts existing logic from `src/commands/update.rs`. `GitUpdater` handles git/GitHub skills by fetching latest commits. The CLI command dispatches to the right updater per source type. Pinned (has `rev`), path, and HTTP skills are skipped.

**Tech Stack:** Rust, ion-skill crate, git CLI, anyhow, sha2

---

### Task 1: Add `default_branch()` to `git.rs`

**Files:**
- Modify: `crates/ion-skill/src/git.rs`

**Step 1: Write the test**

Add to the `#[cfg(test)] mod tests` block at the bottom of `git.rs`:

```rust
#[test]
fn default_branch_of_fresh_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // Init a repo with a commit so HEAD exists
    std::process::Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
    std::process::Command::new("git").args(["commit", "--allow-empty", "-m", "init"]).current_dir(repo).output().unwrap();
    let branch = default_branch(repo).unwrap();
    // Git default is usually "main" or "master"
    assert!(branch == "main" || branch == "master", "got: {branch}");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill default_branch_of_fresh_repo`
Expected: FAIL — `default_branch` not defined

**Step 3: Write the implementation**

Add this function to `git.rs` above the `#[cfg(test)]` block:

```rust
/// Get the default branch name for a repo by checking `origin/HEAD` or falling back
/// to `symbolic-ref HEAD`.
pub fn default_branch(repo_path: &Path) -> Result<String> {
    // Try origin/HEAD first (works for cloned repos)
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git symbolic-ref: {e}")))?;

    if output.status.success() {
        let full_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // refs/remotes/origin/main -> main
        if let Some(branch) = full_ref.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: local HEAD's branch name
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git symbolic-ref: {e}")))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    Err(Error::Git("Could not determine default branch".to_string()))
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -p ion-skill default_branch_of_fresh_repo`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/git.rs
git commit -m "feat: add default_branch() to git module"
```

---

### Task 2: Add `reset_to_remote_head()` to `git.rs`

**Files:**
- Modify: `crates/ion-skill/src/git.rs`

We need a function that resets the working tree to the latest remote HEAD after a fetch, so that the repo reflects the newest state of the default branch.

**Step 1: Write the test**

```rust
#[test]
fn reset_to_remote_head_after_clone() {
    let tmp = tempfile::tempdir().unwrap();

    // Create an "upstream" repo with a commit
    let upstream = tmp.path().join("upstream");
    std::fs::create_dir(&upstream).unwrap();
    std::process::Command::new("git").args(["init"]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "--allow-empty", "-m", "first"]).current_dir(&upstream).output().unwrap();

    // Clone it
    let clone_dir = tmp.path().join("clone");
    clone_or_fetch(&upstream.display().to_string(), &clone_dir).unwrap();
    let commit1 = head_commit(&clone_dir).unwrap();

    // Add a new commit upstream
    std::process::Command::new("git").args(["commit", "--allow-empty", "-m", "second"]).current_dir(&upstream).output().unwrap();

    // Fetch and reset
    clone_or_fetch(&upstream.display().to_string(), &clone_dir).unwrap();
    reset_to_remote_head(&clone_dir).unwrap();
    let commit2 = head_commit(&clone_dir).unwrap();

    assert_ne!(commit1, commit2, "HEAD should have advanced");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill reset_to_remote_head_after_clone`
Expected: FAIL — `reset_to_remote_head` not defined

**Step 3: Write the implementation**

```rust
/// Reset the working tree to the remote's default branch HEAD.
/// Call this after `clone_or_fetch()` to advance to the latest commit.
pub fn reset_to_remote_head(repo_path: &Path) -> Result<()> {
    let branch = default_branch(repo_path)?;
    let remote_ref = format!("origin/{branch}");

    let output = Command::new("git")
        .args(["reset", "--hard", &remote_ref])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git reset: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("git reset --hard {remote_ref} failed: {stderr}")));
    }
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -p ion-skill reset_to_remote_head_after_clone`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/git.rs
git commit -m "feat: add reset_to_remote_head() to git module"
```

---

### Task 3: Create the `update` module with `Updater` trait

**Files:**
- Create: `crates/ion-skill/src/update/mod.rs`
- Modify: `crates/ion-skill/src/lib.rs` (add `pub mod update;`)

**Step 1: Create the module with trait and types**

Create `crates/ion-skill/src/update/mod.rs`:

```rust
pub mod binary;
pub mod git;

use std::path::Path;

use crate::lockfile::LockedSkill;
use crate::manifest::ManifestOptions;
use crate::source::SkillSource;

/// Information about an available update.
#[derive(Debug)]
pub struct UpdateInfo {
    /// Human-readable description of the old version (e.g., commit SHA prefix or version string).
    pub old_version: String,
    /// Human-readable description of the new version.
    pub new_version: String,
}

/// Context passed to updaters for performing the update.
pub struct UpdateContext<'a> {
    pub project_dir: &'a Path,
    pub options: &'a ManifestOptions,
}

/// Trait for source-type-specific update logic.
pub trait Updater {
    /// Check if an update is available. Returns `Some(UpdateInfo)` if yes, `None` if up to date.
    fn check(&self, skill: &LockedSkill, source: &SkillSource) -> crate::Result<Option<UpdateInfo>>;

    /// Apply the update: fetch new version, validate, deploy, return updated lock entry.
    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        ctx: &UpdateContext,
    ) -> crate::Result<LockedSkill>;
}
```

**Step 2: Register the module**

In `crates/ion-skill/src/lib.rs`, add `pub mod update;` after the existing module declarations.

**Step 3: Verify it compiles**

Run: `cargo check -p ion-skill`
Expected: Should compile (binary.rs and git.rs submodules don't exist yet but are declared — this will fail).

Actually, create empty placeholder files first:

Create `crates/ion-skill/src/update/binary.rs`:
```rust
// BinaryUpdater implementation — next task
```

Create `crates/ion-skill/src/update/git.rs`:
```rust
// GitUpdater implementation — next task
```

Run: `cargo check -p ion-skill`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/ion-skill/src/update/ crates/ion-skill/src/lib.rs
git commit -m "feat: add update module with Updater trait"
```

---

### Task 4: Implement `GitUpdater`

**Files:**
- Modify: `crates/ion-skill/src/update/git.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::LockedSkill;
    use crate::source::{SkillSource, SourceType};
    use crate::update::{UpdateContext, Updater};
    use crate::manifest::ManifestOptions;

    fn make_git_repo(path: &std::path::Path) {
        use std::process::Command;
        Command::new("git").args(["init"]).current_dir(path).output().unwrap();
        Command::new("git").args(["commit", "--allow-empty", "-m", "init"]).current_dir(path).output().unwrap();
    }

    fn add_commit(path: &std::path::Path, msg: &str) {
        use std::process::Command;
        Command::new("git").args(["commit", "--allow-empty", "-m", msg]).current_dir(path).output().unwrap();
    }

    #[test]
    fn check_detects_new_commit() {
        let tmp = tempfile::tempdir().unwrap();

        // Create upstream repo
        let upstream = tmp.path().join("upstream");
        std::fs::create_dir(&upstream).unwrap();
        make_git_repo(&upstream);
        let old_commit = crate::git::head_commit(&upstream).unwrap();

        // Clone it to simulate cached repo
        let clone = tmp.path().join("clone");
        crate::git::clone_or_fetch(&upstream.display().to_string(), &clone).unwrap();

        // Create a SKILL.md in the clone so validation can work
        std::fs::write(
            clone.join("SKILL.md"),
            "---\nname: test\ndescription: A test skill.\n---\n\nBody.\n",
        ).unwrap();

        // Add a new commit upstream
        add_commit(&upstream, "second commit");

        let source = SkillSource {
            source_type: SourceType::Git,
            source: upstream.display().to_string(),
            path: None,
            rev: None,
            version: None,
            binary: None,
        };

        let locked = LockedSkill {
            name: "test".to_string(),
            source: upstream.display().to_string(),
            path: None,
            version: None,
            commit: Some(old_commit),
            checksum: None,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        };

        let updater = GitUpdater;
        let result = updater.check(&locked, &source).unwrap();
        assert!(result.is_some(), "should detect new commit");
    }

    #[test]
    fn check_returns_none_when_up_to_date() {
        let tmp = tempfile::tempdir().unwrap();

        let upstream = tmp.path().join("upstream");
        std::fs::create_dir(&upstream).unwrap();
        make_git_repo(&upstream);
        let current_commit = crate::git::head_commit(&upstream).unwrap();

        let clone = tmp.path().join("clone");
        crate::git::clone_or_fetch(&upstream.display().to_string(), &clone).unwrap();

        let source = SkillSource {
            source_type: SourceType::Git,
            source: upstream.display().to_string(),
            path: None,
            rev: None,
            version: None,
            binary: None,
        };

        let locked = LockedSkill {
            name: "test".to_string(),
            source: upstream.display().to_string(),
            path: None,
            version: None,
            commit: Some(current_commit),
            checksum: None,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        };

        let updater = GitUpdater;
        let result = updater.check(&locked, &source).unwrap();
        assert!(result.is_none(), "should be up to date");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib -p ion-skill update::git`
Expected: FAIL — `GitUpdater` not defined

**Step 3: Write the implementation**

Replace `crates/ion-skill/src/update/git.rs` with:

```rust
use std::path::PathBuf;

use crate::installer::{data_dir, hash_simple};
use crate::lockfile::LockedSkill;
use crate::source::SkillSource;
use crate::validate;
use crate::skill::SkillMetadata;
use crate::{git, Error};

use super::{UpdateContext, UpdateInfo, Updater};

pub struct GitUpdater;

impl Updater for GitUpdater {
    fn check(&self, skill: &LockedSkill, source: &SkillSource) -> crate::Result<Option<UpdateInfo>> {
        let url = source.git_url()?;
        let repo_hash = format!("{:x}", hash_simple(&url));
        let repo_dir = data_dir().join(&repo_hash);

        git::clone_or_fetch(&url, &repo_dir)?;
        git::reset_to_remote_head(&repo_dir)?;

        let new_commit = git::head_commit(&repo_dir)?;
        let old_commit = skill.commit.as_deref().unwrap_or("unknown");

        if new_commit == old_commit {
            return Ok(None);
        }

        Ok(Some(UpdateInfo {
            old_version: short_sha(old_commit),
            new_version: short_sha(&new_commit),
        }))
    }

    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        ctx: &UpdateContext,
    ) -> crate::Result<LockedSkill> {
        let url = source.git_url()?;
        let repo_hash = format!("{:x}", hash_simple(&url));
        let repo_dir = data_dir().join(&repo_hash);

        // Repo should already be fetched from check(), but be safe
        git::clone_or_fetch(&url, &repo_dir)?;
        git::reset_to_remote_head(&repo_dir)?;

        let skill_dir = resolve_skill_dir(&repo_dir, source)?;

        // Validate SKILL.md exists and parse it
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Err(Error::InvalidSkill(format!(
                "No SKILL.md found at {}",
                skill_md.display()
            )));
        }
        let (meta, body) = SkillMetadata::from_file(&skill_md)?;

        // Run full validation
        let report = validate::validate_skill_dir(&skill_dir, &meta, &body);
        if report.error_count > 0 {
            return Err(Error::ValidationFailed {
                error_count: report.error_count,
                warning_count: report.warning_count,
                info_count: report.info_count,
                report,
            });
        }

        // Deploy symlinks
        deploy_skill(&skill.name, &skill_dir, ctx)?;

        // Build updated lock entry
        let new_commit = git::head_commit(&repo_dir)?;
        let new_checksum = git::checksum_dir(&skill_dir).ok();

        Ok(LockedSkill {
            name: skill.name.clone(),
            source: url,
            path: source.path.clone(),
            version: meta.version().map(|s| s.to_string()),
            commit: Some(new_commit),
            checksum: new_checksum,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        })
    }
}

fn resolve_skill_dir(repo_dir: &std::path::Path, source: &SkillSource) -> crate::Result<PathBuf> {
    match &source.path {
        Some(path) => {
            let skill_dir = repo_dir.join(path);
            if skill_dir.exists() {
                return Ok(skill_dir);
            }
            let fallback = repo_dir.join("skills").join(path);
            if fallback.exists() {
                return Ok(fallback);
            }
            Err(Error::Source(format!(
                "Skill path '{path}' not found in repository"
            )))
        }
        None => Ok(repo_dir.to_path_buf()),
    }
}

fn deploy_skill(name: &str, skill_dir: &std::path::Path, ctx: &UpdateContext) -> crate::Result<()> {
    use crate::installer::SkillInstaller;

    let installer = SkillInstaller::new(ctx.project_dir, ctx.options);
    // We reuse deploy by calling it through the installer's public interface.
    // Since deploy is private, we need to make it accessible or duplicate the symlink logic.
    // For now, use the create_skill_symlink pattern directly.

    let agents_target = ctx.project_dir.join(".agents").join("skills").join(name);
    create_skill_symlink(skill_dir, &agents_target)?;

    let canonical = ctx.project_dir.join(".agents").join("skills").join(name);
    for target_path in ctx.options.targets.values() {
        let target_skill_dir = ctx.project_dir.join(target_path).join(name);
        create_skill_symlink(&canonical, &target_skill_dir)?;
    }
    Ok(())
}

fn create_skill_symlink(original: &std::path::Path, link: &std::path::Path) -> crate::Result<()> {
    if link.is_symlink() {
        std::fs::remove_file(link).map_err(Error::Io)?;
    } else if link.exists() {
        std::fs::remove_dir_all(link).map_err(Error::Io)?;
    }
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    let link_parent = link.parent().unwrap();
    let relative = pathdiff::diff_paths(original, link_parent)
        .ok_or_else(|| Error::Io(std::io::Error::other(
            format!("Cannot compute relative path from {} to {}", link_parent.display(), original.display()),
        )))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&relative, link).map_err(Error::Io)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&relative, link).map_err(Error::Io)?;
    Ok(())
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    // tests from Step 1 go here
}
```

**Important note:** The `deploy_skill` function duplicates the symlink logic from `installer.rs`. An alternative is to make `SkillInstaller::deploy()` public. The implementer should evaluate which is cleaner — if making `deploy()` `pub` is simple and doesn't break encapsulation, prefer that over duplication. If so, replace the `deploy_skill` and `create_skill_symlink` functions with:

```rust
fn deploy_skill(name: &str, skill_dir: &std::path::Path, ctx: &UpdateContext) -> crate::Result<()> {
    let installer = SkillInstaller::new(ctx.project_dir, ctx.options);
    installer.deploy(name, skill_dir)
}
```

And change `deploy` in `installer.rs` from `fn deploy` to `pub fn deploy`.

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib -p ion-skill update::git`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/update/git.rs crates/ion-skill/src/installer.rs
git commit -m "feat: implement GitUpdater for non-binary skill updates"
```

---

### Task 5: Implement `BinaryUpdater`

**Files:**
- Modify: `crates/ion-skill/src/update/binary.rs`

This extracts the existing binary update logic from `src/commands/update.rs` into the trait.

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::LockedSkill;
    use crate::source::{SkillSource, SourceType};
    use crate::update::Updater;

    #[test]
    fn check_returns_none_when_same_version() {
        // BinaryUpdater::check requires network access to GitHub API,
        // so we test the version comparison logic directly.
        // A full integration test will cover the network path.

        // This is a unit-level smoke test: given a locked skill with a version,
        // calling check should attempt a network call. We can't mock it easily,
        // so we just verify the struct exists and the trait is implemented.
        let _updater = BinaryUpdater;
        // Trait bound check at compile time is sufficient.
    }
}
```

**Step 2: Write the implementation**

Replace `crates/ion-skill/src/update/binary.rs`:

```rust
use crate::binary;
use crate::lockfile::LockedSkill;
use crate::source::SkillSource;

use super::{UpdateContext, UpdateInfo, Updater};

pub struct BinaryUpdater;

impl Updater for BinaryUpdater {
    fn check(&self, skill: &LockedSkill, source: &SkillSource) -> crate::Result<Option<UpdateInfo>> {
        let binary_name = source.binary.as_deref().unwrap_or(&skill.name);
        let current_version = skill
            .binary_version
            .as_deref()
            .unwrap_or("unknown")
            .to_string();

        let release = binary::fetch_github_release(&source.source, source.rev.as_deref())?;
        let latest_version = binary::parse_version_from_tag(&release.tag_name).to_string();

        if current_version == latest_version {
            return Ok(None);
        }

        Ok(Some(UpdateInfo {
            old_version: format!("v{current_version}"),
            new_version: format!("v{latest_version}"),
        }))
    }

    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        ctx: &UpdateContext,
    ) -> crate::Result<LockedSkill> {
        let binary_name = source.binary.as_deref().unwrap_or(&skill.name);
        let current_version = skill.binary_version.as_deref().unwrap_or("unknown");

        let skill_dir = ctx
            .project_dir
            .join(".agents")
            .join("skills")
            .join(&skill.name);

        let result = binary::install_binary_from_github(
            &source.source,
            binary_name,
            source.rev.as_deref(),
            &skill_dir,
        )?;

        // Clean up old version
        if current_version != "unknown" && current_version != result.version {
            let _ = binary::remove_binary_version(binary_name, current_version);
        }

        // Build updated lock entry preserving non-binary fields
        let mut updated = skill.clone();
        updated.binary_version = Some(result.version);
        updated.binary_checksum = Some(result.binary_checksum);
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_updater_implements_trait() {
        let _updater = BinaryUpdater;
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p ion-skill`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/ion-skill/src/update/binary.rs
git commit -m "feat: implement BinaryUpdater with Updater trait"
```

---

### Task 6: Rewrite `src/commands/update.rs` to use the `Updater` trait

**Files:**
- Modify: `src/commands/update.rs`

**Step 1: Rewrite the command**

Replace the entire `src/commands/update.rs` with:

```rust
use ion_skill::manifest::Manifest;
use ion_skill::source::SourceType;
use ion_skill::update::{UpdateContext, UpdateInfo, Updater};
use ion_skill::update::binary::BinaryUpdater;
use ion_skill::update::git::GitUpdater;

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;
    let options = ctx.merged_options(&manifest);

    let update_ctx = UpdateContext {
        project_dir: &ctx.project_dir,
        options: &options,
    };

    // Collect skills eligible for update
    let skills: Vec<(String, _)> = manifest
        .skills
        .iter()
        .filter(|(skill_name, _)| {
            name.is_none() || name == Some(skill_name.as_str())
        })
        .filter_map(|(skill_name, entry)| {
            let source = Manifest::resolve_entry(entry).ok()?;
            Some((skill_name.clone(), source))
        })
        .collect();

    if skills.is_empty() {
        if let Some(n) = name {
            anyhow::bail!("No skill '{}' found in Ion.toml", n);
        }
        println!("No skills to update.");
        return Ok(());
    }

    println!("Updating skills...");

    let mut updated_count = 0u32;
    let mut skipped_count = 0u32;
    let mut failed_count = 0u32;
    let mut up_to_date_count = 0u32;

    for (skill_name, source) in &skills {
        // Skip path and HTTP skills
        if matches!(source.source_type, SourceType::Path | SourceType::Http) {
            continue;
        }

        // Skip pinned skills (those with a rev set) for non-binary types
        if source.rev.is_some() && source.source_type != SourceType::Binary {
            skipped_count += 1;
            println!(
                "  {} {}  {}",
                p.dim("-"),
                p.bold(skill_name),
                p.dim(&format!("skipped (pinned to {})", source.rev.as_deref().unwrap()))
            );
            continue;
        }

        // Pick the right updater
        let updater: Box<dyn Updater> = match source.source_type {
            SourceType::Binary => Box::new(BinaryUpdater),
            SourceType::Github | SourceType::Git => Box::new(GitUpdater),
            _ => continue,
        };

        // Check for updates
        let update_info = match updater.check(
            lockfile.find(skill_name).unwrap_or(&ion_skill::lockfile::LockedSkill {
                name: skill_name.clone(),
                source: source.source.clone(),
                path: source.path.clone(),
                version: None,
                commit: None,
                checksum: None,
                binary: None,
                binary_version: None,
                binary_checksum: None,
            }),
            source,
        ) {
            Ok(Some(info)) => info,
            Ok(None) => {
                up_to_date_count += 1;
                println!(
                    "  {} {}  {}",
                    p.dim("·"),
                    p.bold(skill_name),
                    p.dim("already up to date")
                );
                continue;
            }
            Err(e) => {
                failed_count += 1;
                println!(
                    "  {} {}  {}",
                    p.warn("✗"),
                    p.bold(skill_name),
                    p.warn(&format!("check failed: {e}"))
                );
                continue;
            }
        };

        // Apply the update
        let locked = lockfile.find(skill_name).cloned().unwrap_or_else(|| {
            ion_skill::lockfile::LockedSkill {
                name: skill_name.clone(),
                source: source.source.clone(),
                path: source.path.clone(),
                version: None,
                commit: None,
                checksum: None,
                binary: None,
                binary_version: None,
                binary_checksum: None,
            }
        });

        match updater.apply(&locked, source, &update_ctx) {
            Ok(new_locked) => {
                lockfile.upsert(new_locked);
                updated_count += 1;
                let suffix = if source.source_type == SourceType::Binary {
                    " (binary)"
                } else {
                    ""
                };
                println!(
                    "  {} {}  {} → {}{}",
                    p.success("✓"),
                    p.bold(skill_name),
                    update_info.old_version,
                    p.info(&update_info.new_version),
                    suffix,
                );
            }
            Err(e) => {
                failed_count += 1;
                println!(
                    "  {} {}  {}",
                    p.warn("✗"),
                    p.bold(skill_name),
                    p.warn(&format!("{e}"))
                );
            }
        }
    }

    // Write lockfile if anything changed
    if updated_count > 0 {
        lockfile.write_to(&ctx.lockfile_path)?;
    }

    // Summary line
    let mut parts = Vec::new();
    if updated_count > 0 {
        parts.push(format!("{updated_count} updated"));
    }
    if skipped_count > 0 {
        parts.push(format!("{skipped_count} skipped"));
    }
    if failed_count > 0 {
        parts.push(format!("{failed_count} failed"));
    }
    if up_to_date_count > 0 {
        parts.push(format!("{up_to_date_count} up to date"));
    }

    if !parts.is_empty() {
        println!("\n{}", parts.join(", "));
    }

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/commands/update.rs
git commit -m "refactor: rewrite update command to use Updater trait dispatch"
```

---

### Task 7: Integration test — update a git skill

**Files:**
- Modify: `tests/integration.rs` (or create `tests/update_integration.rs`)

**Step 1: Write the integration test**

```rust
#[test]
fn update_git_skill_pulls_latest_commit() {
    let tmp = tempfile::tempdir().unwrap();

    // Create an "upstream" skill repo with SKILL.md
    let upstream = tmp.path().join("upstream");
    std::fs::create_dir(&upstream).unwrap();
    std::process::Command::new("git").args(["init"]).current_dir(&upstream).output().unwrap();
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill for updates.\n---\n\n# Test\n\nOriginal body.\n",
    ).unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(&upstream).output().unwrap();

    // Set up project: add the skill
    let project = tmp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let output = ion_cmd()
        .args(["add", &upstream.display().to_string()])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(output.status.success(), "add failed: {}", String::from_utf8_lossy(&output.stderr));

    // Read the lockfile to get original commit
    let lock_content = std::fs::read_to_string(project.join("Ion.lock")).unwrap();
    let lockfile: ion_skill::lockfile::Lockfile = toml::from_str(&lock_content).unwrap();
    let original_commit = lockfile.skills[0].commit.clone().unwrap();

    // Make a new commit upstream
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: test-skill\ndescription: An updated test skill.\n---\n\n# Test\n\nUpdated body.\n",
    ).unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "-m", "update content"]).current_dir(&upstream).output().unwrap();

    // Run ion update
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "update failed: {}\n{}", stdout, String::from_utf8_lossy(&output.stderr));

    // Verify lockfile was updated
    let lock_content = std::fs::read_to_string(project.join("Ion.lock")).unwrap();
    let lockfile: ion_skill::lockfile::Lockfile = toml::from_str(&lock_content).unwrap();
    let new_commit = lockfile.skills[0].commit.clone().unwrap();
    assert_ne!(original_commit, new_commit, "commit should have changed");
}
```

**Step 2: Run the test**

Run: `cargo test update_git_skill_pulls_latest_commit`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add integration test for git skill update"
```

---

### Task 8: Integration test — pinned rev is skipped

**Files:**
- Modify: `tests/update_integration.rs` (or same file as Task 7)

**Step 1: Write the test**

```rust
#[test]
fn update_skips_pinned_git_skill() {
    let tmp = tempfile::tempdir().unwrap();

    // Create upstream
    let upstream = tmp.path().join("upstream");
    std::fs::create_dir(&upstream).unwrap();
    std::process::Command::new("git").args(["init"]).current_dir(&upstream).output().unwrap();
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: pinned-skill\ndescription: A pinned test skill.\n---\n\n# Test\n\nBody.\n",
    ).unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(&upstream).output().unwrap();

    // Get the commit SHA to pin to
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&upstream)
        .output()
        .unwrap();
    let pin_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Set up project with a pinned rev in Ion.toml
    let project = tmp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(
        project.join("Ion.toml"),
        format!(
            "[skills]\npinned-skill = {{ source = \"{}\", rev = \"{}\" }}\n",
            upstream.display(),
            &pin_commit[..7]
        ),
    ).unwrap();

    // Install first
    let output = ion_cmd()
        .args(["install"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(output.status.success(), "install failed");

    // Make a new commit upstream
    std::process::Command::new("git").args(["commit", "--allow-empty", "-m", "newer"]).current_dir(&upstream).output().unwrap();

    // Run ion update
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("skipped") || stdout.contains("pinned"), "should mention skip: {stdout}");

    // Verify lockfile commit hasn't changed
    let lock_content = std::fs::read_to_string(project.join("Ion.lock")).unwrap();
    let lockfile: ion_skill::lockfile::Lockfile = toml::from_str(&lock_content).unwrap();
    let commit = lockfile.skills[0].commit.clone().unwrap();
    assert_eq!(commit, pin_commit, "pinned skill should not have been updated");
}
```

**Step 2: Run the test**

Run: `cargo test update_skips_pinned_git_skill`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add integration test for pinned skill skip during update"
```

---

### Task 9: Integration test — validation failure preserves old version

**Files:**
- Modify: `tests/update_integration.rs` (or same file)

**Step 1: Write the test**

```rust
#[test]
fn update_preserves_old_version_on_validation_failure() {
    let tmp = tempfile::tempdir().unwrap();

    // Create upstream with valid skill
    let upstream = tmp.path().join("upstream");
    std::fs::create_dir(&upstream).unwrap();
    std::process::Command::new("git").args(["init"]).current_dir(&upstream).output().unwrap();
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: fail-skill\ndescription: A skill that will fail validation later.\n---\n\n# Test\n\nGood body.\n",
    ).unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(&upstream).output().unwrap();

    // Install
    let project = tmp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let output = ion_cmd()
        .args(["add", &upstream.display().to_string()])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(output.status.success());

    let lock_before = std::fs::read_to_string(project.join("Ion.lock")).unwrap();

    // Push an invalid update upstream (prompt injection via zero-width space)
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: fail-skill\ndescription: Now has injection.\n---\n\nHidden instruction \u{200B} marker.\n",
    ).unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(&upstream).output().unwrap();
    std::process::Command::new("git").args(["commit", "-m", "add injection"]).current_dir(&upstream).output().unwrap();

    // Run update — should fail validation but not crash
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    // The command should succeed overall (individual failures don't abort)
    assert!(output.status.success());

    // Lockfile should be unchanged (validation failure = no update)
    let lock_after = std::fs::read_to_string(project.join("Ion.lock")).unwrap();
    assert_eq!(lock_before, lock_after, "lockfile should not have changed");
}
```

**Step 2: Run the test**

Run: `cargo test update_preserves_old_version_on_validation_failure`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add integration test for validation failure during update"
```

---

### Task 10: Final cleanup and full test run

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings

**Step 3: Run fmt**

Run: `cargo fmt`

**Step 4: Final commit if any formatting changes**

```bash
git add -A
git commit -m "chore: format and lint cleanup"
```
