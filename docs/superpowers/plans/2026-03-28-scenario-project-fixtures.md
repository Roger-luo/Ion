# Scenario Project Fixtures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic, template-based project fixture system to the `scenario` crate so CLI tests can declaratively set up project directory structures.

**Architecture:** A `ProjectBuilder` creates a `Project` (backed by a tempdir or caller-provided path) by rendering on-disk template directories through minijinja. Templates declare variables, path mappings, symlinks, and optional files via a `template.toml` manifest. `Project` integrates with `Scenario` via a `.project()` convenience method.

**Tech Stack:** Rust 2024, minijinja (templating), tempfile (tempdirs), toml + serde (manifest parsing), globset (glob matching for optional files)

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `crates/scenario/src/manifest.rs` | `template.toml` parsing: `TemplateManifest`, `VariableDecl`, `FilesConfig` types and deserialization |
| `crates/scenario/src/project.rs` | `Project`, `ProjectDir`, `ProjectBuilder` — builder API, template rendering pipeline, file writing |
| `crates/scenario/tests/project.rs` | Integration tests for the project fixture system |
| `crates/scenario/tests/fixtures/` | Test template directories (created per-task below) |

### Modified Files

| File | Change |
|------|--------|
| `crates/scenario/Cargo.toml` | Add `minijinja`, `tempfile`, `toml`, `serde`, `globset` dependencies |
| `crates/scenario/src/lib.rs` | Add `mod manifest; mod project;` and re-export `Project`, `ProjectBuilder` |
| `crates/scenario/src/error.rs` | Add six new error variants for template/project operations |
| `crates/scenario/src/scenario.rs` | Add `.project(&Project)` method to `Scenario` |

---

### Task 1: Add dependencies, scaffold modules, add error variants

**Files:**
- Modify: `crates/scenario/Cargo.toml`
- Modify: `crates/scenario/src/error.rs`
- Modify: `crates/scenario/src/lib.rs`
- Create: `crates/scenario/src/manifest.rs`
- Create: `crates/scenario/src/project.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

In `crates/scenario/Cargo.toml`, add to `[dependencies]`:

```toml
minijinja = "2"
tempfile = "3"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
globset = "0.4"
```

- [ ] **Step 2: Add error variants to `error.rs`**

Add these variants to the `Error` enum in `crates/scenario/src/error.rs`:

```rust
use std::path::PathBuf;
// (existing imports stay)

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ... existing variants unchanged ...

    /// Template directory does not exist.
    #[error("template not found: {}", path.display())]
    TemplateNotFound { path: PathBuf },

    /// Failed to parse template.toml manifest.
    #[error("failed to parse {}: {source}", path.display())]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    /// Required template variables were not provided.
    #[error("missing required template variable(s): {}", names.join(", "))]
    MissingVariable { names: Vec<String> },

    /// An unknown variable was set via `.var()`.
    #[error("unknown template variable: {name}")]
    UnknownVariable { name: String },

    /// Minijinja failed to render a template file.
    #[error("template render error in {file}: {source}")]
    TemplateRender {
        file: String,
        source: minijinja::Error,
    },

    /// A symlink target does not exist after rendering.
    #[error("symlink target does not exist: {}", path.display())]
    SymlinkTarget { path: PathBuf },
}
```

- [ ] **Step 3: Create empty module files**

Create `crates/scenario/src/manifest.rs`:

```rust
//! Parsing for `template.toml` manifest files.
```

Create `crates/scenario/src/project.rs`:

```rust
//! Project fixture builder for setting up test directory structures.
```

- [ ] **Step 4: Register modules and re-exports in `lib.rs`**

In `crates/scenario/src/lib.rs`, add the new modules and exports:

```rust
mod error;
mod manifest;
mod output;
mod project;
mod scenario;
mod session;

pub use error::Error;
pub use output::Output;
pub use project::{Project, ProjectBuilder};
pub use scenario::{Scenario, Terminal};
pub use session::Session;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p scenario`
Expected: compiles with no errors (warnings about unused imports are fine at this stage)

- [ ] **Step 6: Commit**

```bash
git add crates/scenario/Cargo.toml crates/scenario/src/error.rs crates/scenario/src/lib.rs crates/scenario/src/manifest.rs crates/scenario/src/project.rs
git commit -m "feat(scenario): scaffold project fixture modules and error variants"
```

---

### Task 2: Manifest parsing

**Files:**
- Modify: `crates/scenario/src/manifest.rs`
- Create: `crates/scenario/tests/fixtures/with-manifest/template.toml`
- Create: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write failing tests for manifest parsing**

Create `crates/scenario/tests/project.rs`:

```rust
use std::fs;
use std::path::Path;

// ── Manifest parsing ───────────────────────────────────────────────

#[test]
fn parse_manifest_full() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-manifest");
    let manifest = scenario::manifest::TemplateManifest::from_dir(&dir).unwrap();

    // Variables
    assert!(manifest.variables.contains_key("name"));
    assert_eq!(
        manifest.variables["name"].default.as_deref(),
        Some("test-skill")
    );
    assert!(manifest.variables.contains_key("description"));
    assert!(manifest.variables["description"].default.is_none());

    // Optional files
    assert_eq!(manifest.files.optional, vec!["Ion.lock".to_string()]);

    // Mappings
    assert_eq!(
        manifest.files.mappings.get("skills/SKILL.md").map(|s| s.as_str()),
        Some(".agents/skills/{{name}}/SKILL.md")
    );

    // Symlinks
    assert_eq!(
        manifest
            .files
            .symlinks
            .get(".claude/skills/{{name}}")
            .map(|s| s.as_str()),
        Some("../../.agents/skills/{{name}}")
    );
}

#[test]
fn parse_manifest_minimal() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("template.toml"), "").unwrap();
    let manifest = scenario::manifest::TemplateManifest::from_dir(tmp.path()).unwrap();

    assert!(manifest.variables.is_empty());
    assert!(manifest.files.optional.is_empty());
    assert!(manifest.files.mappings.is_empty());
    assert!(manifest.files.symlinks.is_empty());
}

#[test]
fn parse_manifest_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    // No template.toml
    let result = scenario::manifest::TemplateManifest::from_dir(tmp.path());
    assert!(result.is_err());
}
```

- [ ] **Step 2: Create the test fixture template**

Create `crates/scenario/tests/fixtures/with-manifest/template.toml`:

```toml
[variables]
name = { description = "Skill name", default = "test-skill" }
description = { description = "Skill description" }
version = { default = "1.0" }

[files]
optional = ["Ion.lock"]

[files.mappings]
"skills/SKILL.md" = ".agents/skills/{{name}}/SKILL.md"

[files.symlinks]
".claude/skills/{{name}}" = "../../.agents/skills/{{name}}"
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo nextest run -p scenario -E 'test(parse_manifest)'`
Expected: FAIL — `scenario::manifest::TemplateManifest` doesn't exist yet

- [ ] **Step 4: Implement manifest types and parsing**

Replace `crates/scenario/src/manifest.rs` with:

```rust
//! Parsing for `template.toml` manifest files.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::Error;

/// A parsed `template.toml` manifest.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TemplateManifest {
    /// Declared template variables.
    pub variables: HashMap<String, VariableDecl>,
    /// File configuration: optional files, mappings, symlinks.
    pub files: FilesConfig,
}

/// Declaration of a single template variable.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct VariableDecl {
    /// Human-readable description of the variable.
    pub description: Option<String>,
    /// Default value. If `None`, the variable is required.
    pub default: Option<String>,
}

/// File-related configuration from `template.toml`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct FilesConfig {
    /// Glob patterns for files excluded by default (opt-in via `.include()`).
    pub optional: Vec<String>,
    /// Source path (in template dir) → destination path (rendered).
    pub mappings: HashMap<String, String>,
    /// Symlink path (rendered) → symlink target (rendered).
    pub symlinks: HashMap<String, String>,
}

impl TemplateManifest {
    /// Parse `template.toml` from the given template directory.
    pub fn from_dir(dir: &Path) -> Result<Self, Error> {
        let path = dir.join("template.toml");
        let content = std::fs::read_to_string(&path).map_err(|_| Error::TemplateNotFound {
            path: path.clone(),
        })?;
        let manifest: TemplateManifest =
            toml::from_str(&content).map_err(|source| Error::ManifestParse {
                path: path.clone(),
                source,
            })?;
        Ok(manifest)
    }
}
```

- [ ] **Step 5: Make manifest module public for tests**

In `crates/scenario/src/lib.rs`, change `mod manifest;` to `pub mod manifest;`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(parse_manifest)'`
Expected: all 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/scenario/src/manifest.rs crates/scenario/src/lib.rs crates/scenario/tests/project.rs crates/scenario/tests/fixtures/
git commit -m "feat(scenario): implement template.toml manifest parsing"
```

---

### Task 3: `Project::empty()` with `.file()` and `.dir()`

**Files:**
- Modify: `crates/scenario/src/project.rs`
- Modify: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write failing tests**

Add to `crates/scenario/tests/project.rs`:

```rust
use scenario::Project;

// ── Empty project ──────────────────────────────────────────────────

#[test]
fn empty_project_creates_tempdir() {
    let project = Project::empty().build().unwrap();
    assert!(project.path().exists());
    assert!(project.path().is_dir());
}

#[test]
fn empty_project_with_file() {
    let project = Project::empty()
        .file("config.toml", "[settings]\nkey = \"value\"")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("config.toml")).unwrap();
    assert_eq!(content, "[settings]\nkey = \"value\"");
}

#[test]
fn empty_project_with_nested_file() {
    let project = Project::empty()
        .file("a/b/c.txt", "deep")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("a/b/c.txt")).unwrap();
    assert_eq!(content, "deep");
}

#[test]
fn empty_project_with_dir() {
    let project = Project::empty().dir("empty-dir").build().unwrap();

    assert!(project.path().join("empty-dir").is_dir());
}

#[test]
fn empty_project_cleanup_on_drop() {
    let path;
    {
        let project = Project::empty()
            .file("tmp.txt", "gone soon")
            .build()
            .unwrap();
        path = project.path().to_path_buf();
        assert!(path.exists());
    }
    // After drop, the tempdir should be cleaned up
    assert!(!path.exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p scenario -E 'test(empty_project)'`
Expected: FAIL — `Project` struct doesn't exist yet

- [ ] **Step 3: Implement `Project`, `ProjectDir`, and `ProjectBuilder` core**

Replace `crates/scenario/src/project.rs` with:

```rust
//! Project fixture builder for setting up test directory structures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Error;

/// A materialized project directory, ready for use in a [`Scenario`](crate::Scenario).
///
/// When created via [`build()`](ProjectBuilder::build), the project lives in a
/// temporary directory that is automatically cleaned up when `Project` is dropped.
/// When created via [`build_in()`](ProjectBuilder::build_in), the caller manages
/// the directory lifecycle.
///
/// # Example
///
/// ```no_run
/// use scenario::Project;
///
/// let project = Project::empty()
///     .file("config.toml", "[settings]\nkey = \"value\"")
///     .build()
///     .unwrap();
///
/// assert!(project.path().join("config.toml").exists());
/// ```
pub struct Project {
    dir: ProjectDir,
}

enum ProjectDir {
    Temp(tempfile::TempDir),
    External(PathBuf),
}

impl Project {
    /// Path to the materialized project directory.
    pub fn path(&self) -> &Path {
        match &self.dir {
            ProjectDir::Temp(tmp) => tmp.path(),
            ProjectDir::External(p) => p,
        }
    }

    /// Start building a project from a template directory on disk.
    ///
    /// The template directory must contain a `template.toml` manifest.
    /// See the crate documentation for the manifest format.
    pub fn from_template(path: impl AsRef<Path>) -> ProjectBuilder {
        ProjectBuilder {
            source: BuilderSource::Template(path.as_ref().to_path_buf()),
            vars: HashMap::new(),
            excludes: Vec::new(),
            includes: Vec::new(),
            overrides: HashMap::new(),
            extra_files: Vec::new(),
            extra_dirs: Vec::new(),
        }
    }

    /// Start building an empty project (no template).
    ///
    /// Use `.file()` and `.dir()` to add content. Files are written verbatim
    /// (no template rendering).
    pub fn empty() -> ProjectBuilder {
        ProjectBuilder {
            source: BuilderSource::Empty,
            vars: HashMap::new(),
            excludes: Vec::new(),
            includes: Vec::new(),
            overrides: HashMap::new(),
            extra_files: Vec::new(),
            extra_dirs: Vec::new(),
        }
    }
}

enum BuilderSource {
    Empty,
    Template(PathBuf),
}

/// Builder for creating a [`Project`] fixture.
pub struct ProjectBuilder {
    source: BuilderSource,
    vars: HashMap<String, String>,
    excludes: Vec<String>,
    includes: Vec<String>,
    overrides: HashMap<String, String>,
    extra_files: Vec<(String, String)>,
    extra_dirs: Vec<String>,
}

impl ProjectBuilder {
    /// Set a template variable.
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple template variables.
    pub fn vars<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.vars.insert(k.into(), v.into());
        }
        self
    }

    /// Exclude a file or directory from the template output.
    ///
    /// Uses exact path matching. For directories, matches any file with this prefix.
    pub fn exclude(mut self, path: impl Into<String>) -> Self {
        self.excludes.push(path.into());
        self
    }

    /// Re-include a file or directory that is optional by default in the template.
    ///
    /// Uses exact path matching. For directories, matches any file with this prefix.
    pub fn include(mut self, path: impl Into<String>) -> Self {
        self.includes.push(path.into());
        self
    }

    /// Replace a template file with custom content.
    ///
    /// The content is still rendered through the template engine, so it can
    /// reference template variables.
    pub fn override_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.overrides.insert(path.into(), content.into());
        self
    }

    /// Add a file not in the template. Written verbatim (no template rendering).
    pub fn file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.extra_files.push((path.into(), content.into()));
        self
    }

    /// Create an empty directory.
    pub fn dir(mut self, path: impl Into<String>) -> Self {
        self.extra_dirs.push(path.into());
        self
    }

    /// Build the project into a new temporary directory.
    ///
    /// The returned [`Project`] owns the tempdir. It is cleaned up when
    /// the `Project` is dropped.
    pub fn build(self) -> Result<Project, Error> {
        let tmp = tempfile::tempdir()?;
        self.build_into(tmp.path())?;
        Ok(Project {
            dir: ProjectDir::Temp(tmp),
        })
    }

    /// Build the project into an existing directory.
    ///
    /// The caller is responsible for managing the directory lifecycle.
    pub fn build_in(self, path: impl AsRef<Path>) -> Result<Project, Error> {
        let path = path.as_ref().to_path_buf();
        self.build_into(&path)?;
        Ok(Project {
            dir: ProjectDir::External(path),
        })
    }

    fn build_into(self, target: &Path) -> Result<(), Error> {
        match self.source {
            BuilderSource::Empty => {
                // For empty projects, just write extra files and dirs
            }
            BuilderSource::Template(_) => {
                // Template rendering — implemented in later tasks
                todo!("template rendering not yet implemented");
            }
        }

        // Write extra files (verbatim, no rendering)
        for (path, content) in &self.extra_files {
            let dest = target.join(path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
        }

        // Create extra dirs
        for dir in &self.extra_dirs {
            std::fs::create_dir_all(target.join(dir))?;
        }

        Ok(())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(empty_project)'`
Expected: all 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/src/project.rs crates/scenario/tests/project.rs
git commit -m "feat(scenario): implement Project::empty() with file and dir support"
```

---

### Task 4: Template loading with variable validation and basic rendering

**Files:**
- Modify: `crates/scenario/src/project.rs`
- Modify: `crates/scenario/tests/project.rs`
- Create: `crates/scenario/tests/fixtures/basic/template.toml`
- Create: `crates/scenario/tests/fixtures/basic/greeting.txt`

- [ ] **Step 1: Create the test fixture**

Create `crates/scenario/tests/fixtures/basic/template.toml`:

```toml
[variables]
name = { description = "A name", default = "world" }
greeting = { description = "Greeting word" }
```

Create `crates/scenario/tests/fixtures/basic/greeting.txt`:

```
{{ greeting }}, {{ name }}!
```

- [ ] **Step 2: Write failing tests**

Add to `crates/scenario/tests/project.rs`:

```rust
use scenario::Error;

// ── Template: basic rendering ──────────────────────────────────────

#[test]
fn template_basic_rendering() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hello, world!\n");
}

#[test]
fn template_override_default() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hi")
        .var("name", "Rust")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hi, Rust!\n");
}

#[test]
fn template_missing_required_var() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let result = Project::from_template(&fixtures).build();

    match result {
        Err(Error::MissingVariable { names }) => {
            assert_eq!(names, vec!["greeting"]);
        }
        other => panic!("expected MissingVariable error, got: {other:?}"),
    }
}

#[test]
fn template_unknown_var() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let result = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .var("typo_var", "oops")
        .build();

    match result {
        Err(Error::UnknownVariable { name }) => {
            assert_eq!(name, "typo_var");
        }
        other => panic!("expected UnknownVariable error, got: {other:?}"),
    }
}

#[test]
fn template_not_found() {
    let result = Project::from_template("/nonexistent/path").build();

    assert!(matches!(result, Err(Error::TemplateNotFound { .. })));
}

#[test]
fn template_excludes_manifest_from_output() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build()
        .unwrap();

    // template.toml itself should NOT appear in the output
    assert!(!project.path().join("template.toml").exists());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo nextest run -p scenario -E 'test(template_)'`
Expected: FAIL — `todo!()` panic in `build_into`

- [ ] **Step 4: Implement template rendering in `build_into`**

In `crates/scenario/src/project.rs`, add this import at the top:

```rust
use crate::manifest::TemplateManifest;
```

Replace the `BuilderSource::Template(_)` branch in `build_into` and add the helper methods. Replace the entire `build_into` method:

```rust
    fn build_into(self, target: &Path) -> Result<(), Error> {
        if let BuilderSource::Template(ref template_dir) = self.source {
            let template_dir = template_dir.clone();
            if !template_dir.exists() {
                return Err(Error::TemplateNotFound {
                    path: template_dir,
                });
            }

            let manifest = TemplateManifest::from_dir(&template_dir)?;

            // Validate variables
            let mut missing = Vec::new();
            for (name, decl) in &manifest.variables {
                if decl.default.is_none() && !self.vars.contains_key(name) {
                    missing.push(name.clone());
                }
            }
            if !missing.is_empty() {
                missing.sort();
                return Err(Error::MissingVariable { names: missing });
            }

            for name in self.vars.keys() {
                if !manifest.variables.contains_key(name) {
                    return Err(Error::UnknownVariable {
                        name: name.clone(),
                    });
                }
            }

            // Build minijinja context: defaults, then user vars
            let mut context = HashMap::new();
            for (name, decl) in &manifest.variables {
                if let Some(default) = &decl.default {
                    context.insert(name.clone(), default.clone());
                }
            }
            for (name, value) in &self.vars {
                context.insert(name.clone(), value.clone());
            }

            // Walk template dir, collect files (excluding template.toml)
            let template_files = walk_template_dir(&template_dir)?;

            // Compute file set with optional/include/exclude filtering
            let file_set =
                self.compute_file_set(&template_files, &manifest)?;

            // Create minijinja environment
            let env = minijinja::Environment::new();

            // Render and write each file
            for rel_path in &file_set {
                let source_path = template_dir.join(rel_path);

                // Determine destination: mapped or natural
                let dest_template = manifest
                    .files
                    .mappings
                    .get(rel_path.as_str())
                    .cloned()
                    .unwrap_or_else(|| rel_path.clone());

                // Check for overrides
                let content_template = if let Some(override_content) = self.overrides.get(rel_path.as_str()) {
                    override_content.clone()
                } else {
                    std::fs::read_to_string(&source_path)?
                };

                // Render destination path
                let dest_rendered = env
                    .render_str(&dest_template, &context)
                    .map_err(|source| Error::TemplateRender {
                        file: rel_path.clone(),
                        source,
                    })?;

                // Render content
                let content_rendered = env
                    .render_str(&content_template, &context)
                    .map_err(|source| Error::TemplateRender {
                        file: rel_path.clone(),
                        source,
                    })?;

                // Write file
                let dest = target.join(&dest_rendered);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, content_rendered)?;
            }

            // Create symlinks
            for (link_template, target_template) in &manifest.files.symlinks {
                let link_rendered = env
                    .render_str(link_template, &context)
                    .map_err(|source| Error::TemplateRender {
                        file: link_template.clone(),
                        source,
                    })?;
                let target_rendered = env
                    .render_str(target_template, &context)
                    .map_err(|source| Error::TemplateRender {
                        file: link_template.clone(),
                        source,
                    })?;

                let link_path = target.join(&link_rendered);
                if let Some(parent) = link_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let abs_target = link_path.parent().unwrap().join(&target_rendered);
                if !abs_target.exists() {
                    return Err(Error::SymlinkTarget { path: abs_target });
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&target_rendered, &link_path)?;
                #[cfg(windows)]
                {
                    if abs_target.is_dir() {
                        std::os::windows::fs::symlink_dir(&target_rendered, &link_path)?;
                    } else {
                        std::os::windows::fs::symlink_file(&target_rendered, &link_path)?;
                    }
                }
            }
        }

        // Write extra files (verbatim, no rendering)
        for (path, content) in &self.extra_files {
            let dest = target.join(path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
        }

        // Create extra dirs
        for dir in &self.extra_dirs {
            std::fs::create_dir_all(target.join(dir))?;
        }

        Ok(())
    }

    /// Compute the set of template files to include, applying optional/include/exclude rules.
    fn compute_file_set(
        &self,
        all_files: &[String],
        manifest: &TemplateManifest,
    ) -> Result<Vec<String>, Error> {
        let optional_globs = if manifest.files.optional.is_empty() {
            None
        } else {
            let mut builder = globset::GlobSetBuilder::new();
            for pattern in &manifest.files.optional {
                builder.add(
                    globset::Glob::new(pattern)
                        .map_err(|e| Error::TemplateRender {
                            file: "template.toml".into(),
                            source: minijinja::Error::new(
                                minijinja::ErrorKind::SyntaxError,
                                format!("invalid glob in optional: {e}"),
                            ),
                        })?,
                );
            }
            Some(builder.build().map_err(|e| Error::TemplateRender {
                file: "template.toml".into(),
                source: minijinja::Error::new(
                    minijinja::ErrorKind::SyntaxError,
                    format!("invalid glob set: {e}"),
                ),
            })?)
        };

        let mut result = Vec::new();
        for file in all_files {
            let is_optional = optional_globs
                .as_ref()
                .is_some_and(|gs| gs.is_match(file));

            let explicitly_included = self
                .includes
                .iter()
                .any(|inc| file == inc || file.starts_with(&format!("{inc}/")));

            let explicitly_excluded = self
                .excludes
                .iter()
                .any(|exc| file == exc || file.starts_with(&format!("{exc}/")));

            if explicitly_excluded {
                continue;
            }
            if is_optional && !explicitly_included {
                continue;
            }
            result.push(file.clone());
        }

        Ok(result)
    }
```

Add this free function at the bottom of `project.rs`:

```rust
/// Walk a template directory and return all file paths relative to it,
/// excluding `template.toml`.
fn walk_template_dir(dir: &Path) -> Result<Vec<String>, Error> {
    let mut files = Vec::new();
    walk_recursive(dir, dir, &mut files)?;
    Ok(files)
}

fn walk_recursive(root: &Path, current: &Path, files: &mut Vec<String>) -> Result<(), Error> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(root, &path, files)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("path is under root")
                .to_string_lossy()
                .to_string();
            if rel != "template.toml" {
                files.push(rel);
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(template_)' -E 'test(empty_project)'`
Expected: all tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/scenario/src/project.rs crates/scenario/tests/project.rs crates/scenario/tests/fixtures/basic/
git commit -m "feat(scenario): implement template loading, variable validation, and rendering"
```

---

### Task 5: File filtering (optional, include, exclude)

**Files:**
- Modify: `crates/scenario/tests/project.rs`
- Create: `crates/scenario/tests/fixtures/with-optional/template.toml`
- Create: `crates/scenario/tests/fixtures/with-optional/config.txt`
- Create: `crates/scenario/tests/fixtures/with-optional/lockfile.txt`
- Create: `crates/scenario/tests/fixtures/with-optional/extra/data.txt`

- [ ] **Step 1: Create the test fixture**

Create `crates/scenario/tests/fixtures/with-optional/template.toml`:

```toml
[variables]

[files]
optional = ["lockfile.txt", "extra/*"]
```

Create `crates/scenario/tests/fixtures/with-optional/config.txt`:

```
always included
```

Create `crates/scenario/tests/fixtures/with-optional/lockfile.txt`:

```
optional lock data
```

Create `crates/scenario/tests/fixtures/with-optional/extra/data.txt`:

```
optional extra data
```

- [ ] **Step 2: Write failing tests**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── File filtering ─────────────────────────────────────────────────

#[test]
fn optional_files_excluded_by_default() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures).build().unwrap();

    assert!(project.path().join("config.txt").exists());
    assert!(!project.path().join("lockfile.txt").exists());
    assert!(!project.path().join("extra/data.txt").exists());
}

#[test]
fn include_brings_back_optional() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .include("lockfile.txt")
        .build()
        .unwrap();

    assert!(project.path().join("config.txt").exists());
    assert!(project.path().join("lockfile.txt").exists());
    assert!(!project.path().join("extra/data.txt").exists());
}

#[test]
fn include_dir_prefix_brings_back_all() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .include("extra")
        .build()
        .unwrap();

    assert!(project.path().join("extra/data.txt").exists());
}

#[test]
fn exclude_removes_non_optional() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .exclude("config.txt")
        .build()
        .unwrap();

    assert!(!project.path().join("config.txt").exists());
    assert!(!project.path().join("lockfile.txt").exists());
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(optional_) | test(include_) | test(exclude_)'`
Expected: all 4 tests PASS (the filtering logic was already implemented in Task 4)

- [ ] **Step 4: Commit**

```bash
git add crates/scenario/tests/project.rs crates/scenario/tests/fixtures/with-optional/
git commit -m "test(scenario): add file filtering tests for optional/include/exclude"
```

---

### Task 6: Path mappings

**Files:**
- Modify: `crates/scenario/tests/project.rs`
- Create: `crates/scenario/tests/fixtures/with-mappings/template.toml`
- Create: `crates/scenario/tests/fixtures/with-mappings/skill.md`
- Create: `crates/scenario/tests/fixtures/with-mappings/config.txt`

- [ ] **Step 1: Create the test fixture**

Create `crates/scenario/tests/fixtures/with-mappings/template.toml`:

```toml
[variables]
name = { description = "Skill name" }

[files.mappings]
"skill.md" = ".agents/skills/{{ name }}/SKILL.md"
```

Create `crates/scenario/tests/fixtures/with-mappings/skill.md`:

```
---
name: {{ name }}
description: A test skill
---
Body of {{ name }}
```

Create `crates/scenario/tests/fixtures/with-mappings/config.txt`:

```
unmapped file
```

- [ ] **Step 2: Write tests**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── Path mappings ──────────────────────────────────────────────────

#[test]
fn mapping_routes_source_to_rendered_dest() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-mappings");
    let project = Project::from_template(&fixtures)
        .var("name", "my-skill")
        .build()
        .unwrap();

    // Source file should NOT exist at its natural path
    assert!(!project.path().join("skill.md").exists());

    // It should exist at the mapped, rendered path
    let content =
        fs::read_to_string(project.path().join(".agents/skills/my-skill/SKILL.md")).unwrap();
    assert!(content.contains("name: my-skill"));
    assert!(content.contains("Body of my-skill"));

    // Unmapped files keep their natural path
    assert!(project.path().join("config.txt").exists());
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(mapping_)'`
Expected: PASS (mapping logic was implemented in Task 4)

- [ ] **Step 4: Commit**

```bash
git add crates/scenario/tests/project.rs crates/scenario/tests/fixtures/with-mappings/
git commit -m "test(scenario): add path mapping tests"
```

---

### Task 7: Override files

**Files:**
- Modify: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write tests**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── Overrides ──────────────────────────────────────────────────────

#[test]
fn override_replaces_template_content() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Yo")
        .override_file("greeting.txt", "Custom: {{ name }} says {{ greeting }}\n")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Custom: world says Yo\n");
}

#[test]
fn extra_file_added_verbatim() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hi")
        .file("extra.txt", "{{ not rendered }}")
        .build()
        .unwrap();

    // Extra files are written verbatim — no template rendering
    let content = fs::read_to_string(project.path().join("extra.txt")).unwrap();
    assert_eq!(content, "{{ not rendered }}");
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(override_) | test(extra_file)'`
Expected: PASS (override and extra file logic implemented in Task 4)

- [ ] **Step 3: Commit**

```bash
git add crates/scenario/tests/project.rs
git commit -m "test(scenario): add override and extra file tests"
```

---

### Task 8: Symlinks

**Files:**
- Modify: `crates/scenario/tests/project.rs`
- Create: `crates/scenario/tests/fixtures/with-symlinks/template.toml`
- Create: `crates/scenario/tests/fixtures/with-symlinks/skills/readme.md`

- [ ] **Step 1: Create the test fixture**

Create `crates/scenario/tests/fixtures/with-symlinks/template.toml`:

```toml
[variables]
name = { description = "Skill name" }

[files.mappings]
"skills/readme.md" = ".agents/skills/{{ name }}/readme.md"

[files.symlinks]
".targets/{{ name }}" = "../../.agents/skills/{{ name }}"
```

Create `crates/scenario/tests/fixtures/with-symlinks/skills/readme.md`:

```
Skill: {{ name }}
```

- [ ] **Step 2: Write tests**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── Symlinks ───────────────────────────────────────────────────────

#[test]
fn symlink_created_with_rendered_paths() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-symlinks");
    let project = Project::from_template(&fixtures)
        .var("name", "my-skill")
        .build()
        .unwrap();

    let link_path = project.path().join(".targets/my-skill");
    assert!(link_path.symlink_metadata().unwrap().is_symlink());

    // The symlink should resolve to the real skill directory
    let resolved = fs::read_to_string(link_path.join("readme.md")).unwrap();
    assert!(resolved.contains("Skill: my-skill"));
}

#[test]
fn symlink_missing_target_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(
        template_dir.join("template.toml"),
        r#"
[variables]

[files.symlinks]
"link" = "nonexistent-target"
"#,
    )
    .unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::SymlinkTarget { .. })));
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(symlink_)'`
Expected: PASS (symlink logic implemented in Task 4)

- [ ] **Step 4: Commit**

```bash
git add crates/scenario/tests/project.rs crates/scenario/tests/fixtures/with-symlinks/
git commit -m "test(scenario): add symlink creation tests"
```

---

### Task 9: `build_in()` support

**Files:**
- Modify: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write tests**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── build_in ───────────────────────────────────────────────────────

#[test]
fn build_in_populates_existing_dir() {
    let target = tempfile::tempdir().unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build_in(target.path())
        .unwrap();

    assert_eq!(project.path(), target.path());
    let content = fs::read_to_string(target.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hello, world!\n");
}

#[test]
fn build_in_does_not_cleanup_on_drop() {
    let target = tempfile::tempdir().unwrap();
    let target_path = target.path().to_path_buf();

    {
        let _project = Project::empty()
            .file("test.txt", "data")
            .build_in(&target_path)
            .unwrap();
    }

    // Directory still exists after Project is dropped
    assert!(target_path.join("test.txt").exists());
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(build_in)'`
Expected: PASS (build_in logic implemented in Task 3)

- [ ] **Step 3: Commit**

```bash
git add crates/scenario/tests/project.rs
git commit -m "test(scenario): add build_in tests"
```

---

### Task 10: Scenario `.project()` integration

**Files:**
- Modify: `crates/scenario/src/scenario.rs`
- Modify: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/scenario/tests/project.rs`:

```rust
use scenario::Scenario;

// ── Scenario integration ───────────────────────────────────────────

#[test]
fn scenario_project_sets_current_dir() {
    let project = Project::empty()
        .file("marker.txt", "found it")
        .build()
        .unwrap();

    let output = Scenario::new("cat")
        .arg("marker.txt")
        .project(&project)
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stdout().contains("found it"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p scenario -E 'test(scenario_project)'`
Expected: FAIL — `project()` method doesn't exist on `Scenario`

- [ ] **Step 3: Add `.project()` method to `Scenario`**

In `crates/scenario/src/scenario.rs`, add this import at the top:

```rust
use crate::project::Project;
```

Add this method to the `impl Scenario` block, after the `current_dir` method:

```rust
    /// Set the working directory to a [`Project`]'s path.
    ///
    /// This is convenience sugar for `.current_dir(project.path())`.
    /// The `Project` must outlive the scenario execution.
    pub fn project(self, project: &Project) -> Self {
        self.current_dir(project.path())
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(scenario_project)'`
Expected: PASS

- [ ] **Step 5: Run the full scenario test suite to ensure nothing is broken**

Run: `cargo nextest run -p scenario`
Expected: all tests PASS

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p scenario --all-targets -- -D warnings`
Expected: no errors

- [ ] **Step 7: Commit**

```bash
git add crates/scenario/src/scenario.rs crates/scenario/tests/project.rs
git commit -m "feat(scenario): add Scenario::project() integration method"
```

---

### Task 11: Template render error test

**Files:**
- Modify: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write test for bad template syntax**

Add to `crates/scenario/tests/project.rs`:

```rust
// ── Error cases ────────────────────────────────────────────────────

#[test]
fn template_render_error_on_bad_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join("template.toml"), "[variables]\n").unwrap();
    fs::write(template_dir.join("bad.txt"), "{{ unclosed").unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::TemplateRender { .. })));
}

#[test]
fn malformed_manifest_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join("template.toml"), "[invalid\nbroken").unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::ManifestParse { .. })));
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo nextest run -p scenario -E 'test(template_render_error) | test(malformed_manifest)'`
Expected: PASS

- [ ] **Step 3: Run full test suite and format**

Run: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run -p scenario`
Expected: all pass, no warnings

- [ ] **Step 4: Commit**

```bash
git add crates/scenario/tests/project.rs
git commit -m "test(scenario): add template error case tests"
```

---

### Task 12: Final full-workspace verification

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace build**

Run: `cargo build`
Expected: compiles cleanly

- [ ] **Step 2: Run full workspace tests**

Run: `cargo nextest run`
Expected: all tests pass

- [ ] **Step 3: Run clippy on full workspace**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues
