# Global Configuration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a global `config.toml` with default targets, source aliases, cache settings, and UI options that merge with per-project config.

**Architecture:** New `config` module in `ion-skill` crate provides `GlobalConfig` struct with `load()`/`save()`. CLI commands load global config at startup and merge with project-level `ManifestOptions`. Source alias expansion happens before `SkillSource::infer()`.

**Tech Stack:** Rust, `dirs` crate (already a dependency), `toml`/`serde` for parsing.

---

### Task 1: Create GlobalConfig struct with load/save

**Files:**
- Create: `crates/ion-skill/src/config.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/config.rs` (inline tests)

**Step 1: Write the failing tests**

Create `crates/ion-skill/src/config.rs` with tests first:

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalConfig {
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CacheConfig {
    pub max_age_days: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiConfig {
    pub color: Option<bool>,
}

impl GlobalConfig {
    pub fn config_path() -> Option<PathBuf> {
        todo!()
    }

    pub fn load() -> Result<Self> {
        todo!()
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        todo!()
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = GlobalConfig::load_from(&path).unwrap();
        assert!(config.targets.is_empty());
        assert!(config.sources.is_empty());
        assert_eq!(config.cache.max_age_days, None);
        assert_eq!(config.ui.color, None);
    }

    #[test]
    fn load_parses_all_sections() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, r#"
[targets]
claude = ".claude/skills"
cursor = ".cursor/skills"

[sources]
superpowers = "obra/superpowers"

[cache]
max-age-days = 30

[ui]
color = true
"#).unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.targets.len(), 2);
        assert_eq!(config.targets["claude"], ".claude/skills");
        assert_eq!(config.sources["superpowers"], "obra/superpowers");
        assert_eq!(config.cache.max_age_days, Some(30));
        assert_eq!(config.ui.color, Some(true));
    }

    #[test]
    fn load_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\n").unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.targets.len(), 1);
        assert!(config.sources.is_empty());
        assert_eq!(config.cache.max_age_days, None);
    }

    #[test]
    fn save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = GlobalConfig::default();
        config.targets.insert("claude".to_string(), ".claude/skills".to_string());
        config.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());
        config.cache.max_age_days = Some(7);
        config.ui.color = Some(false);

        config.save_to(&path).unwrap();

        let reloaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(reloaded.targets["claude"], ".claude/skills");
        assert_eq!(reloaded.sources["superpowers"], "obra/superpowers");
        assert_eq!(reloaded.cache.max_age_days, Some(7));
        assert_eq!(reloaded.ui.color, Some(false));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill config`
Expected: FAIL — `todo!()` panics.

**Step 3: Implement the methods**

Replace the `todo!()` bodies in `GlobalConfig`:

```rust
impl GlobalConfig {
    /// Returns the platform-appropriate path for the global config file.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ion").join("config.toml"))
    }

    /// Load global config from the platform default path.
    /// Returns Default if the file doesn't exist.
    pub fn load() -> Result<Self> {
        match Self::config_path() {
            Some(path) => Self::load_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load global config from a specific path.
    /// Returns Default if the file doesn't exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).map_err(Error::TomlParse),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Save global config to a specific path. Creates parent directories.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Manifest(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }
}
```

**Step 4: Register the module**

In `crates/ion-skill/src/lib.rs`, add:

```rust
pub mod config;
```

**Step 5: Run tests**

Run: `cargo test -p ion-skill config`
Expected: All 4 PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/config.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add GlobalConfig struct with load/save"
```

---

### Task 2: Add target merging (global + project)

**Files:**
- Modify: `crates/ion-skill/src/config.rs`
- Test: `crates/ion-skill/src/config.rs` (inline tests)

**Step 1: Write the failing test**

Add to `mod tests` in `crates/ion-skill/src/config.rs`:

```rust
#[test]
fn resolve_targets_merges_global_and_project() {
    let mut global = GlobalConfig::default();
    global.targets.insert("claude".to_string(), ".claude/skills".to_string());
    global.targets.insert("cursor".to_string(), ".cursor/skills".to_string());

    let mut project = crate::manifest::ManifestOptions::default();
    project.targets.insert("claude".to_string(), ".claude/custom".to_string());

    let merged = global.resolve_targets(&project);
    // Project wins on collision
    assert_eq!(merged["claude"], ".claude/custom");
    // Global fills gaps
    assert_eq!(merged["cursor"], ".cursor/skills");
    assert_eq!(merged.len(), 2);
}

#[test]
fn resolve_targets_empty_global() {
    let global = GlobalConfig::default();
    let mut project = crate::manifest::ManifestOptions::default();
    project.targets.insert("claude".to_string(), ".claude/skills".to_string());

    let merged = global.resolve_targets(&project);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged["claude"], ".claude/skills");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill resolve_targets`
Expected: FAIL — method doesn't exist.

**Step 3: Implement `resolve_targets`**

Add to the `impl GlobalConfig` block in `crates/ion-skill/src/config.rs`:

```rust
/// Merge global targets with project targets. Project wins on key collision.
pub fn resolve_targets(&self, project: &crate::manifest::ManifestOptions) -> BTreeMap<String, String> {
    let mut merged = self.targets.clone();
    for (key, value) in &project.targets {
        merged.insert(key.clone(), value.clone());
    }
    merged
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill resolve_targets`
Expected: All PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/config.rs
git commit -m "feat: add target merging (global + project)"
```

---

### Task 3: Add source alias expansion

**Files:**
- Modify: `crates/ion-skill/src/config.rs`
- Test: `crates/ion-skill/src/config.rs` (inline tests)

**Step 1: Write the failing test**

Add to `mod tests` in `crates/ion-skill/src/config.rs`:

```rust
#[test]
fn resolve_source_expands_alias() {
    let mut global = GlobalConfig::default();
    global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

    // "superpowers/brainstorming" -> "obra/superpowers/brainstorming"
    assert_eq!(
        global.resolve_source("superpowers/brainstorming"),
        "obra/superpowers/brainstorming"
    );
}

#[test]
fn resolve_source_passes_through_unknown() {
    let global = GlobalConfig::default();

    // No alias match -> pass through unchanged
    assert_eq!(
        global.resolve_source("obra/superpowers/brainstorming"),
        "obra/superpowers/brainstorming"
    );
}

#[test]
fn resolve_source_passes_through_urls() {
    let mut global = GlobalConfig::default();
    global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

    // URLs are never expanded
    assert_eq!(
        global.resolve_source("https://github.com/org/repo.git"),
        "https://github.com/org/repo.git"
    );
}

#[test]
fn resolve_source_passes_through_paths() {
    let mut global = GlobalConfig::default();
    global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

    assert_eq!(
        global.resolve_source("./local-skill"),
        "./local-skill"
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill resolve_source`
Expected: FAIL — method doesn't exist.

**Step 3: Implement `resolve_source`**

Add to the `impl GlobalConfig` block:

```rust
/// Expand source aliases. If the first segment of a shorthand matches a source
/// alias, replace it with the alias value. URLs and paths pass through unchanged.
pub fn resolve_source(&self, input: &str) -> String {
    // Don't expand URLs or local paths
    if input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with('/')
        || input.starts_with("./")
        || input.starts_with("../")
    {
        return input.to_string();
    }

    // Check if the first segment is an alias
    let segments: Vec<&str> = input.splitn(2, '/').collect();
    if segments.len() == 2 {
        if let Some(expanded) = self.sources.get(segments[0]) {
            return format!("{}/{}", expanded, segments[1]);
        }
    }

    input.to_string()
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill resolve_source`
Expected: All PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/config.rs
git commit -m "feat: add source alias expansion"
```

---

### Task 4: Integrate global config into install command

**Files:**
- Modify: `src/commands/install.rs`
- Test: manual verification (integration)

**Step 1: Update `src/commands/install.rs`**

Replace the current file with:

```rust
use ion_skill::config::GlobalConfig;
use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};

pub fn run() -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory");
    }

    let global_config = GlobalConfig::load()?;
    let manifest = Manifest::from_file(&manifest_path)?;
    let mut lockfile = Lockfile::from_file(&lockfile_path)?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    // Merge global + project targets
    let merged_targets = global_config.resolve_targets(&manifest.options);
    let merged_options = ManifestOptions { targets: merged_targets };

    println!("Installing {} skill(s)...", manifest.skills.len());

    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing '{name}'...");
        let locked = install_skill(&project_dir, name, &source, &merged_options)?;
        lockfile.upsert(locked);
    }

    lockfile.write_to(&lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");

    // Check gitignore for managed directories
    let mut managed_dirs = vec![".agents/".to_string()];
    for path in merged_options.targets.values() {
        let top_level = path.split('/').next().unwrap_or(path);
        let entry = format!("{top_level}/");
        if !managed_dirs.contains(&entry) {
            managed_dirs.push(entry);
        }
    }

    let dir_refs: Vec<&str> = managed_dirs.iter().map(|s| s.as_str()).collect();
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&project_dir, &dir_refs)?;

    if !missing.is_empty() {
        println!("\nThese directories are not in .gitignore:");
        for dir in &missing {
            println!("  {dir}");
        }
        print!("\nAdd them? [Y/n] (press Enter for yes) ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;

        if answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y") {
            let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
            ion_skill::gitignore::append_to_gitignore(&project_dir, &refs)?;
            println!("Updated .gitignore");
        }
    }

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: integrate global config into install command"
```

---

### Task 5: Integrate global config into add command

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Update `src/commands/add.rs`**

Replace the current file with:

```rust
use ion_skill::config::GlobalConfig;
use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    let global_config = GlobalConfig::load()?;

    // Expand source aliases before inferring
    let expanded = global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

    let manifest = if manifest_path.exists() {
        Manifest::from_file(&manifest_path)?
    } else {
        Manifest::empty()
    };

    // Merge global + project targets
    let merged_targets = global_config.resolve_targets(&manifest.options);
    let merged_options = ManifestOptions { targets: merged_targets };

    let locked = install_skill(&project_dir, &name, &source, &merged_options)?;
    println!("  Installed to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    manifest_writer::add_skill(&manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.upsert(locked);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}

fn skill_name_from_source(source: &SkillSource) -> String {
    if let Some(ref path) = source.path {
        path.rsplit('/').next().unwrap_or(path).to_string()
    } else {
        source
            .source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&source.source)
            .to_string()
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat: integrate global config into add command"
```

---

### Task 6: Integrate global config into remove command

**Files:**
- Modify: `src/commands/remove.rs`

**Step 1: Update `src/commands/remove.rs`**

Replace the current file with:

```rust
use ion_skill::config::GlobalConfig;
use ion_skill::installer::uninstall_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};
use ion_skill::manifest_writer;

pub fn run(name: &str) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    let global_config = GlobalConfig::load()?;
    let manifest = Manifest::from_file(&manifest_path)?;
    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    // Merge global + project targets for cleanup
    let merged_targets = global_config.resolve_targets(&manifest.options);
    let merged_options = ManifestOptions { targets: merged_targets };

    println!("Removing skill '{name}'...");

    uninstall_skill(&project_dir, name, &merged_options)?;
    println!("  Removed from .agents/skills/{name}/");

    manifest_writer::remove_skill(&manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.remove(name);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add src/commands/remove.rs
git commit -m "feat: integrate global config into remove command"
```

---

### Task 7: Run full test suite and fix remaining issues

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All PASS.

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 3: Fix any remaining issues**

If any tests fail or clippy reports issues, fix them.

**Step 4: Final commit (if fixes needed)**

```bash
git add -A
git commit -m "fix: address remaining issues from global config integration"
```

---

## Execution notes

- **Task order:** Task 1 must complete first (creates the module). Tasks 2 and 3 depend on Task 1 but are independent of each other. Tasks 4, 5, 6 depend on Tasks 2 and 3. Task 7 is always last.
- **No new dependencies:** Everything uses the existing `dirs` and `toml` crates.
- **Integration tests:** The existing integration tests don't set up a global config, so they exercise the "no config = default" path, which is correct.
- **The `toml::to_string_pretty` function** requires that the struct implements `Serialize`, which we derive via serde.
- **`ManifestOptions` construction in commands:** We construct a new `ManifestOptions { targets: merged_targets }` to pass to `install_skill`/`uninstall_skill`. This avoids changing the `ManifestOptions` struct or those function signatures.
