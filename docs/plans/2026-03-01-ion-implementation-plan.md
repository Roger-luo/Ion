# ion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a package manager for AI agent skills that fetches, validates, and installs skills from GitHub repos, git URLs, HTTP sources, and local paths.

**Architecture:** Cargo workspace with root crate as the CLI binary and `crates/ion-skill` as the library. The library handles all business logic (manifest parsing, skill validation, source resolution, fetching, installation). The CLI is a thin clap layer. We use shell `git` via `std::process::Command` rather than `libgit2` to keep dependencies light.

**Tech Stack:** Rust 2024 edition, clap 4 (derive), toml/toml_edit, serde/serde_yaml, sha2, reqwest (tokio), tempfile (testing)

---

### Task 1: Set Up Workspace Structure

**Files:**
- Modify: `Cargo.toml` (convert to workspace)
- Create: `crates/ion-skill/Cargo.toml`
- Create: `crates/ion-skill/src/lib.rs`
- Modify: `src/main.rs`

**Step 1: Convert root Cargo.toml to workspace**

Replace `Cargo.toml` with:

```toml
[package]
name = "ion"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4.5.60", features = ["string", "derive"] }
ion-skill = { path = "crates/ion-skill" }

[workspace]
members = [".", "crates/ion-skill"]
```

**Step 2: Create ion-skill crate**

Create `crates/ion-skill/Cargo.toml`:

```toml
[package]
name = "ion-skill"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
toml = "0.8"
toml_edit = "0.22"
thiserror = "2"
```

Create `crates/ion-skill/src/lib.rs`:

```rust
pub mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
```

Create `crates/ion-skill/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Invalid skill: {0}")]
    InvalidSkill(String),

    #[error("Source error: {0}")]
    Source(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Manifest error: {0}")]
    Manifest(String),
}
```

**Step 3: Update src/main.rs with basic CLI skeleton**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a skill to the project
    Add {
        /// Skill source (e.g., owner/repo/skill or git URL)
        source: String,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
    },
    /// Remove a skill from the project
    Remove {
        /// Name of the skill to remove
        name: String,
    },
    /// Install all skills from ion.toml
    Install,
    /// List installed skills
    List,
    /// Show detailed info about a skill
    Info {
        /// Skill source or name
        skill: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { source, rev } => {
            println!("Adding skill: {source}");
            if let Some(rev) = &rev {
                println!("  rev: {rev}");
            }
        }
        Commands::Remove { name } => {
            println!("Removing skill: {name}");
        }
        Commands::Install => {
            println!("Installing skills from ion.toml...");
        }
        Commands::List => {
            println!("Listing skills...");
        }
        Commands::Info { skill } => {
            println!("Info for skill: {skill}");
        }
    }
}
```

**Step 4: Verify it compiles and runs**

Run: `cargo build`
Expected: Compiles successfully.

Run: `cargo run -- add anthropics/skills/brainstorming`
Expected: `Adding skill: anthropics/skills/brainstorming`

Run: `cargo run -- add anthropics/skills/brainstorming --rev v1.0`
Expected: `Adding skill: anthropics/skills/brainstorming` and `  rev: v1.0`

Run: `cargo run -- --help`
Expected: Shows usage with add, remove, install, list, info subcommands.

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/ src/main.rs
git commit -m "feat: set up workspace with ion-skill crate and CLI skeleton"
```

---

### Task 2: Source Type Model

**Files:**
- Create: `crates/ion-skill/src/source.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/source.rs` (inline tests)

**Step 1: Write failing tests for source type inference**

In `crates/ion-skill/src/source.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// The type of source a skill is fetched from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Github,
    Git,
    Http,
    Path,
}

/// A fully resolved skill source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub source_type: SourceType,
    pub source: String,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
}

impl SkillSource {
    /// Infer a SkillSource from a raw source string (no explicit type).
    pub fn infer(source: &str) -> Result<Self> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_github_three_segments() {
        let s = SkillSource::infer("anthropics/skills/brainstorming").unwrap();
        assert_eq!(s.source_type, SourceType::Github);
        assert_eq!(s.source, "anthropics/skills");
        assert_eq!(s.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn infer_github_two_segments() {
        let s = SkillSource::infer("org/my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Github);
        assert_eq!(s.source, "org/my-skill");
        assert_eq!(s.path, None);
    }

    #[test]
    fn infer_github_url() {
        let s = SkillSource::infer("https://github.com/org/repo.git").unwrap();
        assert_eq!(s.source_type, SourceType::Github);
        assert_eq!(s.source, "https://github.com/org/repo.git");
    }

    #[test]
    fn infer_git_url() {
        let s = SkillSource::infer("https://gitlab.com/org/repo.git").unwrap();
        assert_eq!(s.source_type, SourceType::Git);
        assert_eq!(s.source, "https://gitlab.com/org/repo.git");
    }

    #[test]
    fn infer_http_url() {
        let s = SkillSource::infer("https://example.com/skill.tar.gz").unwrap();
        assert_eq!(s.source_type, SourceType::Http);
        assert_eq!(s.source, "https://example.com/skill.tar.gz");
    }

    #[test]
    fn infer_local_relative_path() {
        let s = SkillSource::infer("../my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
        assert_eq!(s.source, "../my-skill");
    }

    #[test]
    fn infer_local_absolute_path() {
        let s = SkillSource::infer("/home/user/skills/my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
        assert_eq!(s.source, "/home/user/skills/my-skill");
    }

    #[test]
    fn infer_local_current_dir_path() {
        let s = SkillSource::infer("./my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
        assert_eq!(s.source, "./my-skill");
    }

    #[test]
    fn infer_single_segment_is_error() {
        let result = SkillSource::infer("brainstorming");
        assert!(result.is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill`
Expected: All tests FAIL with `not yet implemented`.

**Step 3: Implement source type inference**

Replace the `todo!()` in `SkillSource::infer`:

```rust
impl SkillSource {
    /// Infer a SkillSource from a raw source string (no explicit type).
    pub fn infer(source: &str) -> Result<Self> {
        // Local paths
        if source.starts_with('/')
            || source.starts_with("./")
            || source.starts_with("../")
        {
            return Ok(Self {
                source_type: SourceType::Path,
                source: source.to_string(),
                path: None,
                rev: None,
                version: None,
            });
        }

        // URLs
        if source.starts_with("https://") || source.starts_with("http://") {
            let source_type = if source.contains("github.com") {
                SourceType::Github
            } else if source.ends_with(".git") {
                SourceType::Git
            } else {
                SourceType::Http
            };
            return Ok(Self {
                source_type,
                source: source.to_string(),
                path: None,
                rev: None,
                version: None,
            });
        }

        // Shorthand: owner/repo or owner/repo/skill-path
        let segments: Vec<&str> = source.split('/').collect();
        match segments.len() {
            2 => Ok(Self {
                source_type: SourceType::Github,
                source: source.to_string(),
                path: None,
                rev: None,
                version: None,
            }),
            3 => Ok(Self {
                source_type: SourceType::Github,
                source: format!("{}/{}", segments[0], segments[1]),
                path: Some(segments[2].to_string()),
                rev: None,
                version: None,
            }),
            _ => Err(Error::Source(format!(
                "Cannot infer source type from: {source}"
            ))),
        }
    }

    /// Build a git clone URL for this source.
    pub fn git_url(&self) -> Result<String> {
        match self.source_type {
            SourceType::Github => {
                let repo = if self.source.starts_with("https://") {
                    return Ok(self.source.clone());
                } else {
                    &self.source
                };
                Ok(format!("https://github.com/{repo}.git"))
            }
            SourceType::Git => Ok(self.source.clone()),
            _ => Err(Error::Source(format!(
                "Source type {:?} has no git URL",
                self.source_type
            ))),
        }
    }
}
```

**Step 4: Export module from lib.rs**

Add to `crates/ion-skill/src/lib.rs`:

```rust
pub mod error;
pub mod source;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All 9 tests PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/source.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add source type model with inference from strings"
```

---

### Task 3: SKILL.md Parsing

**Files:**
- Create: `crates/ion-skill/src/skill.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/skill.rs` (inline tests)

**Step 1: Write failing tests for SKILL.md parsing**

In `crates/ion-skill/src/skill.rs`:

```rust
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::{Error, Result};

/// Parsed SKILL.md frontmatter.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
}

impl SkillMetadata {
    /// Parse SKILL.md content (frontmatter + body).
    pub fn parse(content: &str) -> Result<(Self, String)> {
        todo!()
    }

    /// Parse SKILL.md from a file path.
    pub fn from_file(path: &Path) -> Result<(Self, String)> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(e))?;
        Self::parse(&content)
    }

    /// Get the version from metadata, if present.
    pub fn version(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("version"))
            .map(|s| s.as_str())
    }

    /// Validate the skill name against the spec rules.
    pub fn validate_name(name: &str) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_skill() {
        let content = "---\nname: my-skill\ndescription: A test skill.\n---\n\n# Instructions\n\nDo things.\n";
        let (meta, body) = SkillMetadata::parse(content).unwrap();
        assert_eq!(meta.name, "my-skill");
        assert_eq!(meta.description, "A test skill.");
        assert!(body.contains("# Instructions"));
    }

    #[test]
    fn parse_skill_with_metadata() {
        let content = "---\nname: my-skill\ndescription: A test skill.\nmetadata:\n  author: test-org\n  version: \"1.0\"\n---\n\nBody.\n";
        let (meta, _body) = SkillMetadata::parse(content).unwrap();
        assert_eq!(meta.version(), Some("1.0"));
        assert_eq!(
            meta.metadata.as_ref().unwrap().get("author").unwrap(),
            "test-org"
        );
    }

    #[test]
    fn parse_skill_missing_frontmatter() {
        let content = "# No frontmatter here\n\nJust markdown.\n";
        let result = SkillMetadata::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_skill_missing_name() {
        let content = "---\ndescription: No name field.\n---\n\nBody.\n";
        let result = SkillMetadata::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn validate_good_names() {
        assert!(SkillMetadata::validate_name("pdf-processing").is_ok());
        assert!(SkillMetadata::validate_name("a").is_ok());
        assert!(SkillMetadata::validate_name("data-analysis").is_ok());
        assert!(SkillMetadata::validate_name("code-review").is_ok());
    }

    #[test]
    fn validate_bad_names() {
        assert!(SkillMetadata::validate_name("PDF-Processing").is_err()); // uppercase
        assert!(SkillMetadata::validate_name("-pdf").is_err()); // starts with hyphen
        assert!(SkillMetadata::validate_name("pdf-").is_err()); // ends with hyphen
        assert!(SkillMetadata::validate_name("pdf--processing").is_err()); // consecutive hyphens
        assert!(SkillMetadata::validate_name("").is_err()); // empty
        let long_name = "a".repeat(65);
        assert!(SkillMetadata::validate_name(&long_name).is_err()); // too long
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill`
Expected: New tests FAIL with `not yet implemented`.

**Step 3: Implement SKILL.md parsing and name validation**

Replace the `todo!()`s:

```rust
impl SkillMetadata {
    /// Parse SKILL.md content (frontmatter + body).
    pub fn parse(content: &str) -> Result<(Self, String)> {
        let content = content.trim_start();
        if !content.starts_with("---") {
            return Err(Error::InvalidSkill(
                "SKILL.md must start with YAML frontmatter (---)".to_string(),
            ));
        }

        let after_first = &content[3..];
        let end = after_first
            .find("\n---")
            .ok_or_else(|| Error::InvalidSkill("No closing --- for frontmatter".to_string()))?;

        let yaml = &after_first[..end];
        let body = after_first[end + 4..].trim_start_matches('\n').to_string();

        let meta: SkillMetadata =
            serde_yaml::from_str(yaml).map_err(|e| Error::YamlParse(e))?;

        Self::validate_name(&meta.name)?;

        if meta.description.is_empty() {
            return Err(Error::InvalidSkill(
                "description must not be empty".to_string(),
            ));
        }

        Ok((meta, body))
    }

    /// Validate the skill name against the spec rules.
    pub fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() || name.len() > 64 {
            return Err(Error::InvalidSkill(format!(
                "name must be 1-64 characters, got {}",
                name.len()
            )));
        }
        if name.starts_with('-') || name.ends_with('-') {
            return Err(Error::InvalidSkill(
                "name must not start or end with a hyphen".to_string(),
            ));
        }
        if name.contains("--") {
            return Err(Error::InvalidSkill(
                "name must not contain consecutive hyphens".to_string(),
            ));
        }
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(Error::InvalidSkill(
                "name must contain only lowercase letters, digits, and hyphens".to_string(),
            ));
        }
        Ok(())
    }
}
```

**Step 4: Export from lib.rs**

Update `crates/ion-skill/src/lib.rs` to add `pub mod skill;`.

**Step 5: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All tests PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/skill.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add SKILL.md frontmatter parsing and name validation"
```

---

### Task 4: Manifest (ion.toml) Parsing and Writing

**Files:**
- Create: `crates/ion-skill/src/manifest.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/manifest.rs` (inline tests)

**Step 1: Write failing tests for manifest parsing**

In `crates/ion-skill/src/manifest.rs`:

```rust
use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::{SkillSource, SourceType};
use crate::{Error, Result};

/// A skill entry in the manifest. Supports both string shorthand and table form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillEntry {
    /// String shorthand: `"anthropics/skills/brainstorming"`
    Shorthand(String),
    /// Full table form with explicit fields
    Full {
        #[serde(rename = "type", default)]
        source_type: Option<SourceType>,
        source: String,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

/// Options section of the manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestOptions {
    #[serde(default)]
    pub install_to_claude: bool,
}

/// Parsed ion.toml manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub skills: BTreeMap<String, SkillEntry>,
    #[serde(default)]
    pub options: ManifestOptions,
}

impl Manifest {
    /// Load manifest from a file path.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        Self::parse(&content)
    }

    /// Parse manifest from TOML string.
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content).map_err(|e| Error::TomlParse(e))
    }

    /// Resolve a SkillEntry into a SkillSource.
    pub fn resolve_entry(entry: &SkillEntry) -> Result<SkillSource> {
        match entry {
            SkillEntry::Shorthand(s) => SkillSource::infer(s),
            SkillEntry::Full {
                source_type,
                source,
                version,
                rev,
                path,
            } => {
                let mut resolved = if let Some(st) = source_type {
                    SkillSource {
                        source_type: st.clone(),
                        source: source.clone(),
                        path: path.clone(),
                        rev: None,
                        version: None,
                    }
                } else {
                    SkillSource::infer(source)?
                };
                if let Some(v) = version {
                    resolved.version = Some(v.clone());
                }
                if let Some(r) = rev {
                    resolved.rev = Some(r.clone());
                }
                if path.is_some() {
                    resolved.path = path.clone();
                }
                Ok(resolved)
            }
        }
    }

    /// Create a default empty manifest.
    pub fn empty() -> Self {
        Self {
            skills: BTreeMap::new(),
            options: ManifestOptions::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shorthand_entry() {
        let toml_str = r#"
[skills]
brainstorming = "anthropics/skills/brainstorming"
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        assert!(manifest.skills.contains_key("brainstorming"));
        let source = Manifest::resolve_entry(&manifest.skills["brainstorming"]).unwrap();
        assert_eq!(source.source_type, SourceType::Github);
        assert_eq!(source.source, "anthropics/skills");
        assert_eq!(source.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn parse_full_github_entry() {
        let toml_str = r#"
[skills]
my-tool = { type = "github", source = "org/skills/my-tool", rev = "v2.0" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-tool"]).unwrap();
        assert_eq!(source.source_type, SourceType::Github);
        assert_eq!(source.rev.as_deref(), Some("v2.0"));
    }

    #[test]
    fn parse_full_git_entry() {
        let toml_str = r#"
[skills]
gitlab-skill = { type = "git", source = "https://gitlab.com/org/skills.git", path = "my-skill" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["gitlab-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Git);
        assert_eq!(source.path.as_deref(), Some("my-skill"));
    }

    #[test]
    fn parse_local_path_entry() {
        let toml_str = r#"
[skills]
local-skill = { type = "path", source = "../my-local-skill" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["local-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Path);
    }

    #[test]
    fn parse_options() {
        let toml_str = r#"
[skills]

[options]
install-to-claude = true
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        assert!(manifest.options.install_to_claude);
    }

    #[test]
    fn parse_empty_manifest() {
        let toml_str = "[skills]\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert!(manifest.skills.is_empty());
        assert!(!manifest.options.install_to_claude);
    }

    #[test]
    fn parse_version_entry() {
        let toml_str = r#"
[skills]
my-skill = { type = "github", source = "org/repo/my-skill", version = "1.0" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-skill"]).unwrap();
        assert_eq!(source.version.as_deref(), Some("1.0"));
    }
}
```

**Step 2: Run tests to verify they pass**

Note: This task has no `todo!()`s — the implementation is inline. But we need to export the module first.

Add `pub mod manifest;` to `crates/ion-skill/src/lib.rs`.

Run: `cargo test -p ion-skill`
Expected: All tests PASS.

**Step 3: Commit**

```bash
git add crates/ion-skill/src/manifest.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add ion.toml manifest parsing with shorthand and full entry support"
```

---

### Task 5: Manifest Writing with toml_edit

**Files:**
- Create: `crates/ion-skill/src/manifest_writer.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/manifest_writer.rs` (inline tests)

We use `toml_edit` to modify `ion.toml` while preserving formatting and comments.

**Step 1: Write failing tests**

In `crates/ion-skill/src/manifest_writer.rs`:

```rust
use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};

use crate::source::SkillSource;
use crate::{Error, Result};

/// Add a skill entry to an ion.toml file. Creates the file if it doesn't exist.
pub fn add_skill(manifest_path: &Path, name: &str, source: &SkillSource) -> Result<String> {
    todo!()
}

/// Remove a skill entry from an ion.toml file.
pub fn remove_skill(manifest_path: &Path, name: &str) -> Result<String> {
    todo!()
}

/// Build a TOML representation of a skill source.
fn skill_to_toml(source: &SkillSource) -> Item {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceType;

    fn make_source(source_type: SourceType, source: &str) -> SkillSource {
        SkillSource {
            source_type,
            source: source.to_string(),
            path: None,
            rev: None,
            version: None,
        }
    }

    #[test]
    fn add_skill_to_empty_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let result = add_skill(
            &path,
            "brainstorming",
            &SkillSource::infer("anthropics/skills/brainstorming").unwrap(),
        )
        .unwrap();

        assert!(result.contains("brainstorming"));
        assert!(result.contains("anthropics/skills/brainstorming"));
    }

    #[test]
    fn add_skill_with_rev() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let mut source = SkillSource::infer("org/my-skill").unwrap();
        source.rev = Some("v1.0".to_string());

        let result = add_skill(&path, "my-skill", &source).unwrap();
        assert!(result.contains("rev"));
        assert!(result.contains("v1.0"));
    }

    #[test]
    fn remove_skill_from_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n",
        )
        .unwrap();

        let result = remove_skill(&path, "brainstorming").unwrap();
        assert!(!result.contains("brainstorming"));
    }

    #[test]
    fn remove_nonexistent_skill_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let result = remove_skill(&path, "nonexistent");
        assert!(result.is_err());
    }
}
```

**Step 2: Add tempfile as dev dependency**

In `crates/ion-skill/Cargo.toml`, add:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p ion-skill`
Expected: New tests FAIL with `not yet implemented`.

**Step 4: Implement manifest writing**

```rust
/// Add a skill entry to an ion.toml string. Returns the updated TOML string.
pub fn add_skill(manifest_path: &Path, name: &str, source: &SkillSource) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    // Ensure [skills] table exists
    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    let skills = doc["skills"].as_table_mut().ok_or_else(|| {
        Error::Manifest("[skills] is not a table".to_string())
    })?;

    skills[name] = skill_to_toml(source);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Remove a skill entry from an ion.toml file. Returns the updated TOML string.
pub fn remove_skill(manifest_path: &Path, name: &str) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path).map_err(Error::Io)?;
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    let skills = doc["skills"].as_table_mut().ok_or_else(|| {
        Error::Manifest("[skills] is not a table".to_string())
    })?;

    if !skills.contains_key(name) {
        return Err(Error::Manifest(format!("Skill '{name}' not found in manifest")));
    }

    skills.remove(name);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Build a TOML representation of a skill source.
fn skill_to_toml(source: &SkillSource) -> Item {
    let needs_table = source.rev.is_some() || source.version.is_some() || source.path.is_some();

    if !needs_table {
        // Use string shorthand for simple GitHub sources
        let display = match (&source.source_type, &source.path) {
            (SourceType::Github, Some(path)) => format!("{}/{}", source.source, path),
            _ => source.source.clone(),
        };
        return value(display).into();
    }

    let mut table = toml_edit::InlineTable::new();

    // Only include type if it's not github (github is the default)
    match source.source_type {
        SourceType::Github => {}
        SourceType::Git => { table.insert("type", "git".into()); }
        SourceType::Http => { table.insert("type", "http".into()); }
        SourceType::Path => { table.insert("type", "path".into()); }
    }

    let source_str = match (&source.source_type, &source.path) {
        (SourceType::Github, Some(path)) => format!("{}/{}", source.source, path),
        _ => source.source.clone(),
    };
    table.insert("source", source_str.into());

    if let Some(ref v) = source.version {
        table.insert("version", v.as_str().into());
    }
    if let Some(ref r) = source.rev {
        table.insert("rev", r.as_str().into());
    }
    if let Some(ref p) = source.path {
        if source.source_type != SourceType::Github {
            table.insert("path", p.as_str().into());
        }
    }

    value(table).into()
}
```

Don't forget to add `use crate::source::SourceType;` at the top.

**Step 5: Export from lib.rs**

Add `pub mod manifest_writer;` to `crates/ion-skill/src/lib.rs`.

**Step 6: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All tests PASS.

**Step 7: Commit**

```bash
git add crates/ion-skill/src/manifest_writer.rs crates/ion-skill/src/lib.rs crates/ion-skill/Cargo.toml
git commit -m "feat: add manifest writing with toml_edit for add/remove operations"
```

---

### Task 6: Lockfile Parsing and Writing

**Files:**
- Create: `crates/ion-skill/src/lockfile.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/lockfile.rs` (inline tests)

**Step 1: Write failing tests**

In `crates/ion-skill/src/lockfile.rs`:

```rust
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A single locked skill entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedSkill {
    pub name: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// The full lockfile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(default, rename = "skill")]
    pub skills: Vec<LockedSkill>,
}

impl Lockfile {
    /// Load lockfile from a path. Returns empty lockfile if file doesn't exist.
    pub fn from_file(path: &Path) -> Result<Self> {
        todo!()
    }

    /// Write lockfile to a path.
    pub fn write_to(&self, path: &Path) -> Result<()> {
        todo!()
    }

    /// Find a locked skill by name.
    pub fn find(&self, name: &str) -> Option<&LockedSkill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Add or update a locked skill entry.
    pub fn upsert(&mut self, skill: LockedSkill) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.name == skill.name) {
            *existing = skill;
        } else {
            self.skills.push(skill);
        }
        self.skills.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Remove a locked skill by name.
    pub fn remove(&mut self, name: &str) {
        self.skills.retain(|s| s.name != name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lockfile() {
        let content = r#"
[[skill]]
name = "brainstorming"
source = "https://github.com/anthropics/skills.git"
path = "brainstorming"
version = "1.0"
commit = "abc123"
checksum = "sha256:deadbeef"
"#;
        let lockfile: Lockfile = toml::from_str(content).unwrap();
        assert_eq!(lockfile.skills.len(), 1);
        assert_eq!(lockfile.skills[0].name, "brainstorming");
        assert_eq!(lockfile.skills[0].commit.as_deref(), Some("abc123"));
    }

    #[test]
    fn roundtrip_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill {
            name: "my-skill".to_string(),
            source: "https://github.com/org/repo.git".to_string(),
            path: None,
            version: Some("1.0".to_string()),
            commit: Some("abc123".to_string()),
            checksum: Some("sha256:deadbeef".to_string()),
        });

        lockfile.write_to(&path).unwrap();
        let loaded = Lockfile::from_file(&path).unwrap();
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.skills[0], lockfile.skills[0]);
    }

    #[test]
    fn upsert_updates_existing() {
        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill {
            name: "s".to_string(),
            source: "a".to_string(),
            path: None,
            version: None,
            commit: Some("old".to_string()),
            checksum: None,
        });
        lockfile.upsert(LockedSkill {
            name: "s".to_string(),
            source: "a".to_string(),
            path: None,
            version: None,
            commit: Some("new".to_string()),
            checksum: None,
        });
        assert_eq!(lockfile.skills.len(), 1);
        assert_eq!(lockfile.skills[0].commit.as_deref(), Some("new"));
    }

    #[test]
    fn remove_skill() {
        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill {
            name: "a".to_string(),
            source: "x".to_string(),
            path: None,
            version: None,
            commit: None,
            checksum: None,
        });
        lockfile.remove("a");
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn from_missing_file_returns_empty() {
        let lockfile = Lockfile::from_file(Path::new("/nonexistent/ion.lock")).unwrap();
        assert!(lockfile.skills.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill`
Expected: New tests FAIL with `not yet implemented`.

**Step 3: Implement lockfile operations**

```rust
impl Lockfile {
    pub fn from_file(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).map_err(|e| Error::TomlParse(e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            Error::Manifest(format!("Failed to serialize lockfile: {e}"))
        })?;
        std::fs::write(path, content).map_err(Error::Io)
    }
}
```

**Step 4: Export from lib.rs**

Add `pub mod lockfile;` to `crates/ion-skill/src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All tests PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/lockfile.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add lockfile parsing, writing, and upsert/remove operations"
```

---

### Task 7: Git Fetcher

**Files:**
- Create: `crates/ion-skill/src/git.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Modify: `crates/ion-skill/Cargo.toml` (add sha2)
- Test: `crates/ion-skill/src/git.rs` (inline tests)

This module shells out to `git` via `std::process::Command`.

**Step 1: Write the module with tests**

In `crates/ion-skill/src/git.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Result};

/// Clone a git repository to a target directory. If it already exists, fetch updates.
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()> {
    if target.join(".git").exists() {
        let output = Command::new("git")
            .args(["fetch", "--all"])
            .current_dir(target)
            .output()
            .map_err(|e| Error::Git(format!("Failed to run git fetch: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(format!("git fetch failed: {stderr}")));
        }
    } else {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        let output = Command::new("git")
            .args(["clone", url, &target.display().to_string()])
            .output()
            .map_err(|e| Error::Git(format!("Failed to run git clone: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(format!("git clone failed: {stderr}")));
        }
    }
    Ok(())
}

/// Checkout a specific ref (branch, tag, or commit SHA).
pub fn checkout(repo_path: &Path, rev: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", rev])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git checkout: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("git checkout {rev} failed: {stderr}")));
    }
    Ok(())
}

/// Get the current HEAD commit SHA.
pub fn head_commit(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git rev-parse: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("git rev-parse failed: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Compute a SHA-256 checksum of a directory's contents (all files, sorted).
pub fn checksum_dir(dir: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    collect_files(dir, &mut files)?;
    files.sort();

    for file in &files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        let content = std::fs::read(file).map_err(Error::Io)?;
        hasher.update(&content);
    }

    let hash = hasher.finalize();
    Ok(format!("sha256:{:x}", hash))
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        if path.file_name().map_or(false, |n| n == ".git") {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_dir_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("b.txt"), "world").unwrap();

        let sum1 = checksum_dir(dir.path()).unwrap();
        let sum2 = checksum_dir(dir.path()).unwrap();
        assert_eq!(sum1, sum2);
        assert!(sum1.starts_with("sha256:"));
    }

    #[test]
    fn checksum_dir_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let sum1 = checksum_dir(dir.path()).unwrap();

        std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
        let sum2 = checksum_dir(dir.path()).unwrap();
        assert_ne!(sum1, sum2);
    }
}
```

**Step 2: Add sha2 dependency**

In `crates/ion-skill/Cargo.toml`, add to `[dependencies]`:

```toml
sha2 = "0.10"
```

**Step 3: Export from lib.rs**

Add `pub mod git;` to `crates/ion-skill/src/lib.rs`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All tests PASS. (The git clone/fetch/checkout functions are tested implicitly in later integration tests.)

**Step 5: Commit**

```bash
git add crates/ion-skill/src/git.rs crates/ion-skill/src/lib.rs crates/ion-skill/Cargo.toml
git commit -m "feat: add git operations (clone, fetch, checkout) and directory checksumming"
```

---

### Task 8: Installer

**Files:**
- Create: `crates/ion-skill/src/installer.rs`
- Modify: `crates/ion-skill/src/lib.rs`
- Test: `crates/ion-skill/src/installer.rs` (inline tests)

**Step 1: Write module with tests**

In `crates/ion-skill/src/installer.rs`:

```rust
use std::path::{Path, PathBuf};

use crate::lockfile::{LockedSkill, Lockfile};
use crate::manifest::{Manifest, ManifestOptions};
use crate::skill::SkillMetadata;
use crate::source::{SkillSource, SourceType};
use crate::{Error, Result, git};

/// Where ion caches cloned repositories.
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("repos")
}

/// Install a single skill from a resolved source into a project directory.
pub fn install_skill(
    project_dir: &Path,
    name: &str,
    source: &SkillSource,
    options: &ManifestOptions,
) -> Result<LockedSkill> {
    let skill_dir = fetch_skill(source)?;

    // Validate SKILL.md exists and is valid
    let skill_md = skill_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err(Error::InvalidSkill(format!(
            "No SKILL.md found at {}",
            skill_md.display()
        )));
    }

    let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

    // Version check
    if let Some(ref required_version) = source.version {
        let actual_version = meta.version().unwrap_or("(none)");
        if actual_version != required_version {
            return Err(Error::InvalidSkill(format!(
                "Version mismatch: expected {required_version}, found {actual_version}"
            )));
        }
    }

    // Copy to .agents/skills/<name>/
    let agents_target = project_dir.join(".agents").join("skills").join(name);
    copy_skill_dir(&skill_dir, &agents_target)?;

    // Optionally copy to .claude/skills/<name>/
    if options.install_to_claude {
        let claude_target = project_dir.join(".claude").join("skills").join(name);
        copy_skill_dir(&skill_dir, &claude_target)?;
    }

    // Build locked entry
    let (commit, checksum) = match source.source_type {
        SourceType::Github | SourceType::Git => {
            // The skill_dir may be a subdirectory — get repo root
            let repo_dir = find_repo_root(&skill_dir);
            let commit = git::head_commit(&repo_dir).ok();
            let checksum = git::checksum_dir(&skill_dir).ok();
            (commit, checksum)
        }
        SourceType::Path => {
            let checksum = git::checksum_dir(&skill_dir).ok();
            (None, checksum)
        }
        SourceType::Http => {
            let checksum = git::checksum_dir(&skill_dir).ok();
            (None, checksum)
        }
    };

    let git_url = source.git_url().ok().unwrap_or_else(|| source.source.clone());

    Ok(LockedSkill {
        name: name.to_string(),
        source: git_url,
        path: source.path.clone(),
        version: meta.version().map(|s| s.to_string()),
        commit,
        checksum,
    })
}

/// Fetch a skill source to a local directory. Returns the path to the skill directory.
fn fetch_skill(source: &SkillSource) -> Result<PathBuf> {
    match source.source_type {
        SourceType::Github | SourceType::Git => {
            let url = source.git_url()?;

            // Cache repos by URL hash
            let repo_hash = format!("{:x}", md5_simple(&url));
            let repo_dir = cache_dir().join(&repo_hash);

            git::clone_or_fetch(&url, &repo_dir)?;

            if let Some(ref rev) = source.rev {
                git::checkout(&repo_dir, rev)?;
            }

            match &source.path {
                Some(path) => {
                    let skill_dir = repo_dir.join(path);
                    if !skill_dir.exists() {
                        return Err(Error::Source(format!(
                            "Skill path '{path}' not found in repository"
                        )));
                    }
                    Ok(skill_dir)
                }
                None => Ok(repo_dir),
            }
        }
        SourceType::Path => {
            let path = PathBuf::from(&source.source);
            if !path.exists() {
                return Err(Error::Source(format!(
                    "Local path does not exist: {}",
                    source.source
                )));
            }
            Ok(path)
        }
        SourceType::Http => {
            Err(Error::Source("HTTP source not yet implemented".to_string()))
        }
    }
}

/// Copy a skill directory to a target location (overwriting if it exists).
fn copy_skill_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        std::fs::remove_dir_all(dst).map_err(Error::Io)?;
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(Error::Io)?;
    for entry in std::fs::read_dir(src).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.file_name().map_or(false, |n| n == ".git") {
            continue; // Skip .git directories
        }
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(Error::Io)?;
        }
    }
    Ok(())
}

/// Remove an installed skill from the project directory.
pub fn uninstall_skill(project_dir: &Path, name: &str, options: &ManifestOptions) -> Result<()> {
    let agents_dir = project_dir.join(".agents").join("skills").join(name);
    if agents_dir.exists() {
        std::fs::remove_dir_all(&agents_dir).map_err(Error::Io)?;
    }
    if options.install_to_claude {
        let claude_dir = project_dir.join(".claude").join("skills").join(name);
        if claude_dir.exists() {
            std::fs::remove_dir_all(&claude_dir).map_err(Error::Io)?;
        }
    }
    Ok(())
}

/// Find the git repo root from a path (walk up looking for .git).
fn find_repo_root(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return current;
        }
        if !current.pop() {
            return path.to_path_buf();
        }
    }
}

/// Simple hash for cache directory naming (not cryptographic).
fn md5_simple(s: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_skill_dir_works() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "---\nname: test\ndescription: Test.\n---\nBody").unwrap();
        std::fs::create_dir(src.path().join("scripts")).unwrap();
        std::fs::write(src.path().join("scripts").join("run.sh"), "#!/bin/bash").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("test-skill");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(dst.join("scripts").join("run.sh").exists());
    }

    #[test]
    fn copy_skill_dir_skips_git() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "content").unwrap();
        std::fs::create_dir(src.path().join(".git")).unwrap();
        std::fs::write(src.path().join(".git").join("HEAD"), "ref").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("out");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(!dst.join(".git").exists());
    }

    #[test]
    fn uninstall_removes_dirs() {
        let project = tempfile::tempdir().unwrap();
        let agents = project.path().join(".agents").join("skills").join("test");
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("SKILL.md"), "x").unwrap();

        let claude = project.path().join(".claude").join("skills").join("test");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(claude.join("SKILL.md"), "x").unwrap();

        let options = ManifestOptions { install_to_claude: true };
        uninstall_skill(project.path(), "test", &options).unwrap();

        assert!(!agents.exists());
        assert!(!claude.exists());
    }

    #[test]
    fn install_local_skill() {
        // Create a local skill directory
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: local-test\ndescription: A local test skill.\n---\n\nInstructions here.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };
        let options = ManifestOptions { install_to_claude: false };

        let locked = install_skill(project.path(), "local-test", &source, &options).unwrap();
        assert_eq!(locked.name, "local-test");
        assert!(project.path().join(".agents/skills/local-test/SKILL.md").exists());
    }
}
```

**Step 2: Add dirs dependency**

In `crates/ion-skill/Cargo.toml`, add to `[dependencies]`:

```toml
dirs = "6"
```

**Step 3: Export from lib.rs**

Add `pub mod installer;` to `crates/ion-skill/src/lib.rs`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/installer.rs crates/ion-skill/src/lib.rs crates/ion-skill/Cargo.toml
git commit -m "feat: add skill installer with local path support, copy, and uninstall"
```

---

### Task 9: Wire Up CLI `add` Command

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/add.rs`
- Modify: `src/main.rs`

**Step 1: Create commands module**

`src/commands/mod.rs`:

```rust
pub mod add;
```

`src/commands/add.rs`:

```rust
use std::path::PathBuf;

use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    // Parse source
    let mut source = SkillSource::infer(source_str)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    // Determine skill name (last segment of path, or last segment of source)
    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

    // Load manifest (create if doesn't exist)
    let manifest = if manifest_path.exists() {
        Manifest::from_file(&manifest_path)?
    } else {
        Manifest::empty()
    };

    // Install the skill
    let locked = install_skill(&project_dir, &name, &source, &manifest.options)?;
    println!("  Installed to .agents/skills/{name}/");
    if manifest.options.install_to_claude {
        println!("  Installed to .claude/skills/{name}/");
    }

    // Update manifest
    manifest_writer::add_skill(&manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    // Update lockfile
    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.upsert(locked);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}

fn skill_name_from_source(source: &SkillSource) -> String {
    if let Some(ref path) = source.path {
        // Use the last segment of the path
        path.rsplit('/').next().unwrap_or(path).to_string()
    } else {
        // Use the last segment of the source
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

**Step 2: Update main.rs**

```rust
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a skill to the project
    Add {
        /// Skill source (e.g., owner/repo/skill or git URL)
        source: String,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
    },
    /// Remove a skill from the project
    Remove {
        /// Name of the skill to remove
        name: String,
    },
    /// Install all skills from ion.toml
    Install,
    /// List installed skills
    List,
    /// Show detailed info about a skill
    Info {
        /// Skill source or name
        skill: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { source, rev } => {
            commands::add::run(&source, rev.as_deref())
        }
        Commands::Remove { name } => {
            println!("Removing skill: {name}");
            Ok(())
        }
        Commands::Install => {
            println!("Installing skills from ion.toml...");
            Ok(())
        }
        Commands::List => {
            println!("Listing skills...");
            Ok(())
        }
        Commands::Info { skill } => {
            println!("Info for skill: {skill}");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

**Step 3: Add anyhow dependency to root Cargo.toml**

Add to `[dependencies]` in `Cargo.toml`:

```toml
anyhow = "1"
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 5: Manual smoke test with a local skill**

```bash
# Create a test skill directory
mkdir -p /tmp/test-skill
cat > /tmp/test-skill/SKILL.md << 'EOF'
---
name: test-skill
description: A test skill for smoke testing.
---

# Test Skill

Do test things.
EOF

# Run ion add with the local path
cd /tmp/test-project && mkdir -p /tmp/test-project
cargo run --manifest-path /path/to/ion/Cargo.toml -- add /tmp/test-skill
```

Expected: Skill gets installed to `/tmp/test-project/.agents/skills/test-skill/`.

**Step 6: Commit**

```bash
git add src/ Cargo.toml Cargo.lock
git commit -m "feat: wire up ion add command with full install pipeline"
```

---

### Task 10: Wire Up CLI `remove` Command

**Files:**
- Create: `src/commands/remove.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Implement remove command**

`src/commands/remove.rs`:

```rust
use ion_skill::installer::uninstall_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;

pub fn run(name: &str) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    // Load manifest
    let manifest = Manifest::from_file(&manifest_path)?;
    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    println!("Removing skill '{name}'...");

    // Uninstall files
    uninstall_skill(&project_dir, name, &manifest.options)?;
    println!("  Removed from .agents/skills/{name}/");

    // Update manifest
    manifest_writer::remove_skill(&manifest_path, name)?;
    println!("  Updated ion.toml");

    // Update lockfile
    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    lockfile.remove(name);
    lockfile.write_to(&lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
```

**Step 2: Update mod.rs and main.rs**

Add `pub mod remove;` to `src/commands/mod.rs`.

In `main.rs`, replace the `Commands::Remove` arm:

```rust
Commands::Remove { name } => {
    commands::remove::run(&name)
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/commands/remove.rs src/commands/mod.rs src/main.rs
git commit -m "feat: wire up ion remove command"
```

---

### Task 11: Wire Up CLI `install` Command

**Files:**
- Create: `src/commands/install.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Implement install command**

`src/commands/install.rs`:

```rust
use ion_skill::installer::install_skill;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;

pub fn run() -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory");
    }

    let manifest = Manifest::from_file(&manifest_path)?;
    let mut lockfile = Lockfile::from_file(&lockfile_path)?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Installing {} skill(s)...", manifest.skills.len());

    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing '{name}'...");

        let locked = install_skill(&project_dir, name, &source, &manifest.options)?;
        lockfile.upsert(locked);
    }

    lockfile.write_to(&lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");
    Ok(())
}
```

**Step 2: Update mod.rs and main.rs**

Add `pub mod install;` to `src/commands/mod.rs`.

In `main.rs`, replace the `Commands::Install` arm:

```rust
Commands::Install => {
    commands::install::run()
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/commands/install.rs src/commands/mod.rs src/main.rs
git commit -m "feat: wire up ion install command"
```

---

### Task 12: Wire Up CLI `list` Command

**Files:**
- Create: `src/commands/list.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Implement list command**

`src/commands/list.rs`:

```rust
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::Manifest;

pub fn run() -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory");
    }

    let manifest = Manifest::from_file(&manifest_path)?;
    let lockfile = Lockfile::from_file(&lockfile_path)?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Skills ({}):", manifest.skills.len());
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        let locked = lockfile.find(name);

        let version_str = locked
            .and_then(|l| l.version.as_deref())
            .unwrap_or("(unknown)");
        let commit_str = locked
            .and_then(|l| l.commit.as_deref())
            .map(|c| &c[..c.len().min(8)])
            .unwrap_or("(none)");

        let installed = project_dir
            .join(".agents")
            .join("skills")
            .join(name)
            .exists();
        let status = if installed { "installed" } else { "not installed" };

        println!(
            "  {name} v{version_str} ({commit_str}) [{status}]"
        );
        println!("    source: {}", source.source);
    }
    Ok(())
}
```

**Step 2: Update mod.rs and main.rs**

Add `pub mod list;` to `src/commands/mod.rs`.

In `main.rs`, replace the `Commands::List` arm:

```rust
Commands::List => {
    commands::list::run()
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/commands/list.rs src/commands/mod.rs src/main.rs
git commit -m "feat: wire up ion list command"
```

---

### Task 13: Wire Up CLI `info` Command

**Files:**
- Create: `src/commands/info.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Implement info command**

`src/commands/info.rs`:

```rust
use ion_skill::manifest::Manifest;
use ion_skill::skill::SkillMetadata;
use ion_skill::source::SkillSource;

pub fn run(skill_str: &str) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");

    // First check if it's a name in the manifest
    if manifest_path.exists() {
        let manifest = Manifest::from_file(&manifest_path)?;
        if let Some(entry) = manifest.skills.get(skill_str) {
            let source = Manifest::resolve_entry(entry)?;
            return show_info_from_installed(&project_dir, skill_str);
        }
    }

    // Otherwise try to resolve as a source
    let source = SkillSource::infer(skill_str)?;
    println!("Fetching info for '{skill_str}'...");
    println!("  Source type: {:?}", source.source_type);
    println!("  Source: {}", source.source);
    if let Some(ref path) = source.path {
        println!("  Path: {path}");
    }
    if let Ok(url) = source.git_url() {
        println!("  Git URL: {url}");
    }
    Ok(())
}

fn show_info_from_installed(project_dir: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let skill_md = project_dir
        .join(".agents")
        .join("skills")
        .join(name)
        .join("SKILL.md");

    if !skill_md.exists() {
        anyhow::bail!("Skill '{name}' is in ion.toml but not installed. Run `ion install`.");
    }

    let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

    println!("Skill: {}", meta.name);
    println!("Description: {}", meta.description);
    if let Some(ref license) = meta.license {
        println!("License: {license}");
    }
    if let Some(ref compat) = meta.compatibility {
        println!("Compatibility: {compat}");
    }
    if let Some(version) = meta.version() {
        println!("Version: {version}");
    }
    if let Some(ref metadata) = meta.metadata {
        for (k, v) in metadata {
            if k != "version" {
                println!("  {k}: {v}");
            }
        }
    }
    Ok(())
}
```

**Step 2: Update mod.rs and main.rs**

Add `pub mod info;` to `src/commands/mod.rs`.

In `main.rs`, replace the `Commands::Info` arm:

```rust
Commands::Info { skill } => {
    commands::info::run(&skill)
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/commands/info.rs src/commands/mod.rs src/main.rs
git commit -m "feat: wire up ion info command"
```

---

### Task 14: Integration Test

**Files:**
- Create: `tests/integration.rs`

**Step 1: Write integration test using a local skill**

`tests/integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn add_and_remove_local_skill() {
    let project = tempfile::tempdir().unwrap();
    let skill_dir = tempfile::tempdir().unwrap();

    // Create a valid skill
    std::fs::write(
        skill_dir.path().join("SKILL.md"),
        "---\nname: test-skill\ndescription: Integration test skill.\nmetadata:\n  version: \"1.0\"\n---\n\n# Test\n\nDo things.\n",
    ).unwrap();

    // ion add
    let output = ion_cmd()
        .args(["add", &skill_dir.path().display().to_string()])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "add failed: {stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(project.path().join(".agents/skills/test-skill/SKILL.md").exists());
    assert!(project.path().join("ion.toml").exists());
    assert!(project.path().join("ion.lock").exists());

    // ion list
    let output = ion_cmd()
        .args(["list"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("test-skill"));

    // ion remove
    let output = ion_cmd()
        .args(["remove", "test-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "remove failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(!project.path().join(".agents/skills/test-skill").exists());
}

#[test]
fn install_from_manifest() {
    let project = tempfile::tempdir().unwrap();
    let skill_dir = tempfile::tempdir().unwrap();

    // Create a valid skill
    std::fs::write(
        skill_dir.path().join("SKILL.md"),
        "---\nname: manifest-skill\ndescription: Manifest test.\n---\n\nBody.\n",
    ).unwrap();

    // Write ion.toml manually
    std::fs::write(
        project.path().join("ion.toml"),
        format!(
            "[skills]\nmanifest-skill = {{ type = \"path\", source = \"{}\" }}\n",
            skill_dir.path().display()
        ),
    ).unwrap();

    // ion install
    let output = ion_cmd()
        .args(["install"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "install failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(project.path().join(".agents/skills/manifest-skill/SKILL.md").exists());
}
```

**Step 2: Add tempfile as root dev dependency**

In root `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run integration tests**

Run: `cargo test --test integration`
Expected: All tests PASS.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All unit tests and integration tests PASS.

**Step 5: Commit**

```bash
git add tests/ Cargo.toml Cargo.lock
git commit -m "test: add integration tests for add, remove, install, and list"
```

---

### Task 15: Final Cleanup and Verify

**Step 1: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests PASS.

**Step 3: Verify help text**

Run: `cargo run -- --help`
Expected: Shows all commands with descriptions.

**Step 4: Fix any clippy warnings or test failures found**

**Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and final cleanup"
```
