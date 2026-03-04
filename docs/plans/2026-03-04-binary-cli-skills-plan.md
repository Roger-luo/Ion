# Binary CLI Skills Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add binary CLI skill support to Ion — download platform-specific binaries from GitHub Releases, generate SKILL.md via `<binary> skill`, and provide `ion run` as the invocation interface.

**Architecture:** Extend the existing `SourceType` enum with a `Binary` variant. A new `crates/ion-skill/src/binary.rs` module handles platform detection, GitHub Release API queries, asset matching, download, extraction, and versioned storage at `~/.local/share/ion/bin/`. The installer delegates to this module for binary sources before running the standard SKILL.md validation and symlink deployment pipeline. A new `ion run` command resolves binary paths from the lockfile and execs the binary.

**Tech Stack:** Rust, reqwest (HTTP/GitHub API), flate2 + tar (extraction), clap (CLI), serde/toml (serialization)

**Design doc:** `docs/plans/2026-03-04-binary-cli-skills-design.md`

---

## Task 1: Add `Binary` variant to `SourceType` and `SkillSource`

**Files:**
- Modify: `crates/ion-skill/src/source.rs:5-23`
- Test: `crates/ion-skill/src/source.rs` (inline tests)

**Step 1: Write the failing test**

Add to the existing test module at the bottom of `source.rs`:

```rust
#[test]
fn test_binary_source_type_serializes() {
    let source = SkillSource {
        source_type: SourceType::Binary,
        source: "owner/mytool".to_string(),
        path: None,
        rev: None,
        version: None,
        binary: Some("mytool".to_string()),
    };
    assert_eq!(source.source_type, SourceType::Binary);
    assert_eq!(source.binary.as_deref(), Some("mytool"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_binary_source_type_serializes`
Expected: FAIL — `Binary` variant and `binary` field don't exist.

**Step 3: Write minimal implementation**

In `source.rs`, add `Binary` to `SourceType` enum (after line 12):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Github,
    Git,
    Http,
    Path,
    Binary,
}
```

Add `binary` field to `SkillSource` (after line 22):

```rust
#[derive(Debug, Clone)]
pub struct SkillSource {
    pub source_type: SourceType,
    pub source: String,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub binary: Option<String>,
}
```

Update all existing construction sites in `SkillSource::infer()` to include `binary: None`. There are 5 return sites in `infer()` (lines ~32, ~40, ~51, ~57, ~71) — each `SkillSource { ... }` needs `binary: None` added.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -p ion-skill test_binary_source_type_serializes`
Expected: PASS

**Step 5: Fix compilation across codebase**

Run: `cargo build 2>&1 | head -40`

Fix any other files that construct `SkillSource` without the `binary` field. Known sites:
- `crates/ion-skill/src/manifest.rs` in `resolve_entry()` (~line 85 and ~line 95) — add `binary: None`
- Any test files constructing `SkillSource`

Run: `cargo build`
Expected: Compiles cleanly.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/source.rs crates/ion-skill/src/manifest.rs
git commit -m "feat: add Binary variant to SourceType and binary field to SkillSource"
```

---

## Task 2: Add `binary` field to `SkillEntry` and `LockedSkill`

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs:9-24`
- Modify: `crates/ion-skill/src/lockfile.rs:7-19`
- Test: inline tests in both files

**Step 1: Write the failing test for manifest parsing**

Create a test in `manifest.rs` (add to existing tests or create test module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_binary_skill_entry() {
        let toml = r#"
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }
"#;
        let manifest = Manifest::parse(toml).unwrap();
        let entry = manifest.skills.get("mytool").unwrap();
        match entry {
            SkillEntry::Full { source_type, source, binary, .. } => {
                assert_eq!(*source_type, Some(SourceType::Binary));
                assert_eq!(source, "owner/mytool");
                assert_eq!(binary.as_deref(), Some("mytool"));
            }
            _ => panic!("Expected Full entry"),
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_parse_binary_skill_entry`
Expected: FAIL — `binary` field doesn't exist on `SkillEntry::Full`.

**Step 3: Add `binary` field to `SkillEntry::Full`**

In `manifest.rs`, add to `SkillEntry::Full` (after the `path` field, ~line 22):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillEntry {
    Shorthand(String),
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
        #[serde(default)]
        binary: Option<String>,
    },
}
```

Update `Manifest::resolve_entry()` to pass `binary` through to `SkillSource`. In the `Full` match arm (~line 86), extract `binary` and set it on the resolved `SkillSource`:

```rust
SkillEntry::Full { source_type, source, version, rev, path, binary } => {
    // ... existing resolution logic ...
    resolved.binary = binary.clone();
    Ok(resolved)
}
```

**Step 4: Run manifest test to verify it passes**

Run: `cargo test --lib -p ion-skill test_parse_binary_skill_entry`
Expected: PASS

**Step 5: Write the failing test for lockfile**

Add to lockfile tests:

```rust
#[test]
fn test_locked_skill_with_binary_fields() {
    let locked = LockedSkill {
        name: "mytool".to_string(),
        source: "https://github.com/owner/mytool.git".to_string(),
        path: None,
        version: Some("1.2.0".to_string()),
        commit: None,
        checksum: None,
        binary: Some("mytool".to_string()),
        binary_version: Some("1.2.0".to_string()),
        binary_checksum: Some("sha256:abc123".to_string()),
    };
    let lockfile = Lockfile { skills: vec![locked] };
    let toml_str = toml::to_string_pretty(&lockfile).unwrap();
    assert!(toml_str.contains("binary = \"mytool\""));
    assert!(toml_str.contains("binary_version = \"1.2.0\""));

    let parsed: Lockfile = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.skills[0].binary.as_deref(), Some("mytool"));
    assert_eq!(parsed.skills[0].binary_version.as_deref(), Some("1.2.0"));
}
```

**Step 6: Run lockfile test to verify it fails**

Run: `cargo test --lib -p ion-skill test_locked_skill_with_binary_fields`
Expected: FAIL — fields don't exist.

**Step 7: Add binary fields to `LockedSkill`**

In `lockfile.rs`, add three fields after `checksum` (~line 18):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_checksum: Option<String>,
}
```

**Step 8: Fix compilation and run all tests**

Run: `cargo build` — fix any sites that construct `LockedSkill` without the new fields (in `installer.rs` `build_locked_entry()` ~line 198). Add `binary: None, binary_version: None, binary_checksum: None` to those sites.

Run: `cargo test --lib -p ion-skill`
Expected: All tests pass.

**Step 9: Commit**

```bash
git add crates/ion-skill/src/manifest.rs crates/ion-skill/src/lockfile.rs crates/ion-skill/src/installer.rs
git commit -m "feat: add binary fields to SkillEntry and LockedSkill"
```

---

## Task 3: Platform detection module

**Files:**
- Create: `crates/ion-skill/src/binary.rs`
- Modify: `crates/ion-skill/src/lib.rs:1-14` (add `pub mod binary;`)

**Step 1: Write the failing test**

Create `crates/ion-skill/src/binary.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
        assert!(!platform.target_triple().is_empty());
        // On macOS arm64, should be "aarch64-apple-darwin"
        // On macOS x86, should be "x86_64-apple-darwin"
        // On Linux x86, should be "x86_64-unknown-linux-gnu"
        let triple = platform.target_triple();
        assert!(triple.contains('-'), "Triple should contain dashes: {}", triple);
    }

    #[test]
    fn test_asset_name_matching() {
        let platform = Platform {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
        };
        let candidates = vec![
            "mytool-x86_64-apple-darwin.tar.gz".to_string(),
            "mytool-aarch64-apple-darwin.tar.gz".to_string(),
            "mytool-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            "checksums.txt".to_string(),
        ];
        let matched = platform.match_asset("mytool", &candidates);
        assert_eq!(matched, Some("mytool-aarch64-apple-darwin.tar.gz".to_string()));
    }

    #[test]
    fn test_asset_name_matching_with_os_arch_format() {
        let platform = Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
        };
        let candidates = vec![
            "mytool-linux-amd64.tar.gz".to_string(),
            "mytool-darwin-arm64.tar.gz".to_string(),
        ];
        let matched = platform.match_asset("mytool", &candidates);
        assert_eq!(matched, Some("mytool-linux-amd64.tar.gz".to_string()));
    }

    #[test]
    fn test_bin_dir() {
        let dir = bin_dir();
        let path_str = dir.to_string_lossy();
        assert!(path_str.contains("ion") && path_str.contains("bin"));
    }

    #[test]
    fn test_binary_path() {
        let path = binary_path("mytool", "1.2.0");
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("mytool"));
        assert!(path_str.contains("1.2.0"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_platform_detection`
Expected: FAIL — module doesn't exist or types missing.

**Step 3: Implement platform detection and asset matching**

Write `crates/ion-skill/src/binary.rs`:

```rust
use std::path::{Path, PathBuf};

/// Detected platform info for binary downloads.
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
    /// Detect the current platform from std::env::consts.
    pub fn detect() -> Self {
        let os = match std::env::consts::OS {
            "macos" => "macos",
            "linux" => "linux",
            "windows" => "windows",
            other => other,
        };
        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86_64" => "x86_64",
            other => other,
        };
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
        }
    }

    /// Build a target triple like "x86_64-apple-darwin".
    pub fn target_triple(&self) -> String {
        let os_part = match self.os.as_str() {
            "macos" => "apple-darwin",
            "linux" => "unknown-linux-gnu",
            "windows" => "pc-windows-msvc",
            other => other,
        };
        format!("{}-{}", self.arch, os_part)
    }

    /// Match a binary name against a list of release asset names.
    /// Returns the best matching asset name.
    pub fn match_asset(&self, binary_name: &str, asset_names: &[String]) -> Option<String> {
        let triple = self.target_triple();

        // Priority 1: exact target triple match (e.g. mytool-aarch64-apple-darwin.tar.gz)
        let triple_pattern = format!("{}-{}", binary_name, triple);
        if let Some(name) = asset_names.iter().find(|n| n.starts_with(&triple_pattern)) {
            return Some(name.clone());
        }

        // Priority 2: OS + arch match with common aliases
        let os_aliases = self.os_aliases();
        let arch_aliases = self.arch_aliases();

        for name in asset_names {
            let lower = name.to_lowercase();
            if !lower.starts_with(&binary_name.to_lowercase()) {
                continue;
            }
            // Skip non-archive files
            if !is_archive(&lower) {
                continue;
            }
            let has_os = os_aliases.iter().any(|a| lower.contains(a));
            let has_arch = arch_aliases.iter().any(|a| lower.contains(a));
            if has_os && has_arch {
                return Some(name.clone());
            }
        }

        None
    }

    fn os_aliases(&self) -> Vec<&str> {
        match self.os.as_str() {
            "macos" => vec!["darwin", "macos", "apple"],
            "linux" => vec!["linux"],
            "windows" => vec!["windows", "win"],
            _ => vec![&self.os],
        }
    }

    fn arch_aliases(&self) -> Vec<&str> {
        match self.arch.as_str() {
            "x86_64" => vec!["x86_64", "amd64", "x64"],
            "aarch64" => vec!["aarch64", "arm64"],
            _ => vec![&self.arch],
        }
    }
}

fn is_archive(name: &str) -> bool {
    name.ends_with(".tar.gz")
        || name.ends_with(".tar.xz")
        || name.ends_with(".zip")
        || name.ends_with(".tgz")
}

/// Root directory for Ion-managed binaries: ~/.local/share/ion/bin/
pub fn bin_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("bin")
}

/// Path to a specific binary version: ~/.local/share/ion/bin/{name}/{version}/{name}
pub fn binary_path(name: &str, version: &str) -> PathBuf {
    bin_dir().join(name).join(version).join(name)
}

/// Path to the "current" symlink: ~/.local/share/ion/bin/{name}/current/{name}
pub fn current_binary_path(name: &str) -> PathBuf {
    bin_dir().join(name).join("current").join(name)
}
```

Add to `crates/ion-skill/src/lib.rs`:

```rust
pub mod binary;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib -p ion-skill binary::tests`
Expected: All 5 tests pass.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/binary.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add platform detection and asset matching for binary skills"
```

---

## Task 4: GitHub Releases API client

**Files:**
- Modify: `crates/ion-skill/src/binary.rs` (add API functions)
- Modify: `crates/ion-skill/Cargo.toml` (add serde_json if not present — already there)

**Step 1: Write the failing test**

Add to `binary.rs` tests:

```rust
#[test]
fn test_parse_github_release_response() {
    let json = r#"{
        "tag_name": "v1.2.0",
        "assets": [
            {"name": "mytool-x86_64-apple-darwin.tar.gz", "browser_download_url": "https://example.com/mytool-x86_64-apple-darwin.tar.gz"},
            {"name": "mytool-aarch64-apple-darwin.tar.gz", "browser_download_url": "https://example.com/mytool-aarch64-apple-darwin.tar.gz"}
        ]
    }"#;
    let release: GitHubRelease = serde_json::from_str(json).unwrap();
    assert_eq!(release.tag_name, "v1.2.0");
    assert_eq!(release.assets.len(), 2);
    assert_eq!(release.assets[0].name, "mytool-x86_64-apple-darwin.tar.gz");
}

#[test]
fn test_parse_version_from_tag() {
    assert_eq!(parse_version_from_tag("v1.2.0"), "1.2.0");
    assert_eq!(parse_version_from_tag("1.2.0"), "1.2.0");
    assert_eq!(parse_version_from_tag("release-1.0"), "release-1.0");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_parse_github_release`
Expected: FAIL — types don't exist.

**Step 3: Implement GitHub Release types and API function**

Add to `binary.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Fetch the latest release (or a specific tag) from GitHub.
/// `repo` is in "owner/repo" format.
pub fn fetch_github_release(repo: &str, tag: Option<&str>) -> crate::Result<GitHubRelease> {
    let url = match tag {
        Some(t) => format!("https://api.github.com/repos/{}/releases/tags/{}", repo, t),
        None => format!("https://api.github.com/repos/{}/releases/latest", repo),
    };

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "ion-skill-manager")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| crate::Error::Other(format!("Failed to fetch release: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::Error::Other(format!(
            "GitHub API returned {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        )));
    }

    resp.json::<GitHubRelease>()
        .map_err(|e| crate::Error::Other(format!("Failed to parse release JSON: {}", e)))
}

/// Strip leading 'v' from tag to get version string.
pub fn parse_version_from_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib -p ion-skill test_parse_github_release test_parse_version_from_tag`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/binary.rs
git commit -m "feat: add GitHub Releases API types and client for binary skills"
```

---

## Task 5: Binary download, extraction, and storage

**Files:**
- Modify: `crates/ion-skill/src/binary.rs` (add download + extract + install)
- Modify: `crates/ion-skill/Cargo.toml` (add `flate2` and `tar` dependencies)

**Step 1: Add dependencies**

In `crates/ion-skill/Cargo.toml`, add:

```toml
flate2 = "1"
tar = "0.4"
zip = "2"
```

**Step 2: Write the failing test**

```rust
#[test]
fn test_install_binary_creates_version_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_root = tmp.path().join("bin");

    // Create a fake binary file
    let fake_binary = tmp.path().join("mytool");
    std::fs::write(&fake_binary, "#!/bin/sh\necho hello").unwrap();

    install_binary_file(&fake_binary, "mytool", "1.2.0", &bin_root).unwrap();

    // Check versioned directory exists
    let installed = bin_root.join("mytool").join("1.2.0").join("mytool");
    assert!(installed.exists(), "Binary should be installed at versioned path");

    // Check current symlink
    let current = bin_root.join("mytool").join("current");
    assert!(current.is_symlink(), "current should be a symlink");
    assert_eq!(
        std::fs::read_link(&current).unwrap(),
        PathBuf::from("1.2.0")
    );
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_install_binary_creates_version_dir`
Expected: FAIL — function doesn't exist.

**Step 4: Implement download, extract, and install functions**

Add to `binary.rs`:

```rust
use std::fs;
use std::io;

/// Download a file from URL to a local path.
pub fn download_file(url: &str, dest: &Path) -> crate::Result<()> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "ion-skill-manager")
        .send()
        .map_err(|e| crate::Error::Other(format!("Download failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::Error::Other(format!(
            "Download returned {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .map_err(|e| crate::Error::Other(format!("Failed to read response: {}", e)))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| crate::Error::Other(format!("Failed to create dir: {}", e)))?;
    }
    fs::write(dest, &bytes)
        .map_err(|e| crate::Error::Other(format!("Failed to write file: {}", e)))?;

    Ok(())
}

/// Extract a .tar.gz archive to a destination directory. Returns list of extracted paths.
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> crate::Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)
        .map_err(|e| crate::Error::Other(format!("Failed to open archive: {}", e)))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    fs::create_dir_all(dest_dir)
        .map_err(|e| crate::Error::Other(format!("Failed to create dest dir: {}", e)))?;

    let mut extracted = Vec::new();
    for entry in archive
        .entries()
        .map_err(|e| crate::Error::Other(format!("Failed to read archive: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| crate::Error::Other(format!("Bad archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| crate::Error::Other(format!("Bad path in archive: {}", e)))?
            .into_owned();
        entry
            .unpack_in(dest_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to extract {}: {}", path.display(), e)))?;
        extracted.push(dest_dir.join(&path));
    }

    Ok(extracted)
}

/// Find the binary executable in an extracted directory.
/// Looks for a file named `binary_name` (or the only executable file).
pub fn find_binary_in_dir(dir: &Path, binary_name: &str) -> crate::Result<PathBuf> {
    // Direct match
    let direct = dir.join(binary_name);
    if direct.is_file() {
        return Ok(direct);
    }

    // Search one level of subdirectories (common: tarball extracts to subdir)
    for entry in fs::read_dir(dir)
        .map_err(|e| crate::Error::Other(format!("Failed to read dir: {}", e)))?
    {
        let entry =
            entry.map_err(|e| crate::Error::Other(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            let nested = path.join(binary_name);
            if nested.is_file() {
                return Ok(nested);
            }
        }
    }

    Err(crate::Error::Other(format!(
        "Could not find binary '{}' in {}",
        binary_name,
        dir.display()
    )))
}

/// Install a binary file to the versioned storage directory.
/// Creates: {bin_root}/{name}/{version}/{name} and {bin_root}/{name}/current -> {version}
pub fn install_binary_file(
    binary_file: &Path,
    name: &str,
    version: &str,
    bin_root: &Path,
) -> crate::Result<()> {
    let version_dir = bin_root.join(name).join(version);
    fs::create_dir_all(&version_dir)
        .map_err(|e| crate::Error::Other(format!("Failed to create version dir: {}", e)))?;

    let dest = version_dir.join(name);
    fs::copy(binary_file, &dest)
        .map_err(|e| crate::Error::Other(format!("Failed to copy binary: {}", e)))?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&dest, perms)
            .map_err(|e| crate::Error::Other(format!("Failed to set permissions: {}", e)))?;
    }

    // Create/update current symlink
    let current_link = bin_root.join(name).join("current");
    if current_link.exists() || current_link.is_symlink() {
        fs::remove_file(&current_link).ok();
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(version, &current_link)
        .map_err(|e| crate::Error::Other(format!("Failed to create current symlink: {}", e)))?;

    Ok(())
}

/// Compute SHA256 checksum of a file.
pub fn file_checksum(path: &Path) -> crate::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path)
        .map_err(|e| crate::Error::Other(format!("Failed to read file for checksum: {}", e)))?;
    let hash = Sha256::digest(&bytes);
    Ok(format!("sha256:{:x}", hash))
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --lib -p ion-skill binary::tests`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/binary.rs crates/ion-skill/Cargo.toml
git commit -m "feat: add binary download, extraction, and versioned storage"
```

---

## Task 6: SKILL.md generation from binary

**Files:**
- Modify: `crates/ion-skill/src/binary.rs` (add `generate_skill_md` function)

**Step 1: Write the failing test**

```rust
#[test]
fn test_generate_skill_md_from_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_path = tmp.path().join("mytool");

    // Create a fake binary that outputs a SKILL.md on `skill` subcommand
    #[cfg(unix)]
    {
        std::fs::write(&bin_path, r#"#!/bin/sh
if [ "$1" = "skill" ]; then
    cat <<'EOF'
---
name: mytool
description: A test tool that does useful things for testing purposes. Invoke with ion run mytool.
metadata:
  binary: mytool
  version: 1.0.0
---
# MyTool

Use `ion run mytool <args>` to run this tool.
EOF
else
    echo "unknown command"
    exit 1
fi
"#).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let skill_md = generate_skill_md(&bin_path).unwrap();
    assert!(skill_md.contains("name: mytool"));
    assert!(skill_md.contains("description:"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_generate_skill_md_from_binary`
Expected: FAIL

**Step 3: Implement `generate_skill_md`**

```rust
use std::process::Command;

/// Run `<binary> skill` and capture the SKILL.md output from stdout.
pub fn generate_skill_md(binary_path: &Path) -> crate::Result<String> {
    let output = Command::new(binary_path)
        .arg("skill")
        .output()
        .map_err(|e| crate::Error::Other(format!(
            "Failed to run '{} skill': {}",
            binary_path.display(),
            e
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::Error::Other(format!(
            "'{}' skill command failed (exit {}): {}",
            binary_path.display(),
            output.status,
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| crate::Error::Other(format!("Invalid UTF-8 in skill output: {}", e)))?;

    if stdout.trim().is_empty() {
        return Err(crate::Error::Other(format!(
            "'{}' skill command produced no output",
            binary_path.display()
        )));
    }

    Ok(stdout)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -p ion-skill test_generate_skill_md_from_binary`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/ion-skill/src/binary.rs
git commit -m "feat: add SKILL.md generation from binary skill subcommand"
```

---

## Task 7: High-level binary install orchestrator

**Files:**
- Modify: `crates/ion-skill/src/binary.rs` (add `install_binary_skill` orchestrator)

This function ties together: GitHub API → download → extract → find binary → install to storage → generate SKILL.md → write to skill dir.

**Step 1: Write the failing test**

```rust
#[test]
fn test_install_binary_skill_with_bundled_skillmd() {
    // Test the "bundled SKILL.md" path without network calls
    let tmp = tempfile::tempdir().unwrap();
    let skill_dir = tmp.path().join("skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // Simulate: binary already in storage, SKILL.md in tarball extract dir
    let extract_dir = tmp.path().join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();
    fs::write(
        extract_dir.join("SKILL.md"),
        "---\nname: mytool\ndescription: A test tool that does useful things for testing purposes and more.\nmetadata:\n  binary: mytool\n  version: 1.0.0\n---\n# MyTool\n",
    )
    .unwrap();

    let result = find_bundled_skill_md(&extract_dir);
    assert!(result.is_some());
    let content = fs::read_to_string(result.unwrap()).unwrap();
    assert!(content.contains("name: mytool"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ion-skill test_install_binary_skill_with_bundled_skillmd`
Expected: FAIL

**Step 3: Implement `find_bundled_skill_md` and `install_binary_skill`**

```rust
/// Look for a SKILL.md in an extracted archive directory.
/// Checks root and one level of subdirectories.
pub fn find_bundled_skill_md(extract_dir: &Path) -> Option<PathBuf> {
    let direct = extract_dir.join("SKILL.md");
    if direct.is_file() {
        return Some(direct);
    }
    // Check subdirectories (tarball often wraps in a dir)
    if let Ok(entries) = fs::read_dir(extract_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let nested = entry.path().join("SKILL.md");
                if nested.is_file() {
                    return Some(nested);
                }
            }
        }
    }
    None
}

/// Full binary skill installation orchestrator.
///
/// 1. Fetch GitHub release (or use provided URL)
/// 2. Download platform-matched asset
/// 3. Extract archive
/// 4. Find and install binary to versioned storage
/// 5. Generate or find SKILL.md
/// 6. Write SKILL.md to skill_dir
///
/// Returns (version, binary_checksum) for lockfile.
pub fn install_binary_from_github(
    repo: &str,
    binary_name: &str,
    rev: Option<&str>,
    skill_dir: &Path,
) -> crate::Result<BinaryInstallResult> {
    let platform = Platform::detect();

    // 1. Fetch release info
    let release = fetch_github_release(repo, rev)?;
    let version = parse_version_from_tag(&release.tag_name).to_string();
    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();

    // 2. Match platform asset
    let asset_name = platform.match_asset(binary_name, &asset_names).ok_or_else(|| {
        crate::Error::Other(format!(
            "No matching release asset for platform {} in {:?}",
            platform.target_triple(),
            asset_names
        ))
    })?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .unwrap();

    // 3. Download to temp
    let tmp_dir = tempfile::tempdir()
        .map_err(|e| crate::Error::Other(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = tmp_dir.path().join(&asset_name);
    download_file(&asset.browser_download_url, &archive_path)?;

    // 4. Extract
    let extract_dir = tmp_dir.path().join("extracted");
    extract_tar_gz(&archive_path, &extract_dir)?;

    // 5. Find and install binary
    let found_binary = find_binary_in_dir(&extract_dir, binary_name)?;
    let bin_root = bin_dir();
    install_binary_file(&found_binary, binary_name, &version, &bin_root)?;

    let installed_binary = binary_path(binary_name, &version);
    let checksum = file_checksum(&installed_binary)?;

    // 6. Generate or find SKILL.md
    fs::create_dir_all(skill_dir)
        .map_err(|e| crate::Error::Other(format!("Failed to create skill dir: {}", e)))?;

    let skill_md_content = if let Some(bundled) = find_bundled_skill_md(&extract_dir) {
        fs::read_to_string(&bundled)
            .map_err(|e| crate::Error::Other(format!("Failed to read bundled SKILL.md: {}", e)))?
    } else {
        generate_skill_md(&installed_binary)?
    };

    fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
        .map_err(|e| crate::Error::Other(format!("Failed to write SKILL.md: {}", e)))?;

    Ok(BinaryInstallResult {
        version,
        binary_checksum: checksum,
    })
}

/// Result from installing a binary skill.
#[derive(Debug)]
pub struct BinaryInstallResult {
    pub version: String,
    pub binary_checksum: String,
}
```

Add `tempfile` to `crates/ion-skill/Cargo.toml` dependencies (move from dev-dependencies):

```toml
tempfile = "3"
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib -p ion-skill binary::tests`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/binary.rs crates/ion-skill/Cargo.toml
git commit -m "feat: add binary skill install orchestrator with bundled SKILL.md fallback"
```

---

## Task 8: Wire binary source into the installer

**Files:**
- Modify: `crates/ion-skill/src/installer.rs` (handle Binary source type in install flow)

**Step 1: Understand current install flow**

The `SkillInstaller::install_with_options()` method at ~line 56 calls:
1. `self.fetch(source)` — clones repo/resolves path
2. `self.validate_spec(&skill_dir, source)` — parses SKILL.md
3. `self.deploy(name, &skill_dir)` — creates symlinks
4. `self.build_locked_entry(name, source, &meta, &skill_dir)` — builds lockfile entry

For binary sources, we need to intercept before `fetch()` and use the binary install pipeline instead.

**Step 2: Write the implementation**

In `installer.rs`, modify `install_with_options()` to handle `SourceType::Binary`:

```rust
pub fn install_with_options(
    &self,
    name: &str,
    source: &SkillSource,
    validation: InstallValidationOptions,
) -> Result<LockedSkill> {
    // Binary sources use a different pipeline
    if source.source_type == SourceType::Binary {
        return self.install_binary(name, source);
    }

    // ... existing code unchanged ...
}
```

Add a new private method `install_binary`:

```rust
fn install_binary(&self, name: &str, source: &SkillSource) -> Result<LockedSkill> {
    use crate::binary;

    let binary_name = source
        .binary
        .as_deref()
        .unwrap_or(name);

    let skill_dir = self.project_dir.join(".agents").join("skills").join(name);

    let result = binary::install_binary_from_github(
        &source.source,
        binary_name,
        source.rev.as_deref(),
        &skill_dir,
    )?;

    // Validate the generated/bundled SKILL.md
    let (meta, _body) = self.validate_spec(&skill_dir, source)?;

    // Deploy symlinks to targets
    self.deploy(name, &skill_dir)?;

    // Build locked entry with binary fields
    Ok(LockedSkill {
        name: name.to_string(),
        source: format!("https://github.com/{}.git", source.source),
        path: source.path.clone(),
        version: meta.version().map(|v| v.to_string()),
        commit: None,
        checksum: None,
        binary: Some(binary_name.to_string()),
        binary_version: Some(result.version),
        binary_checksum: Some(result.binary_checksum),
    })
}
```

Add `use crate::source::SourceType;` at the top of `installer.rs` if not already imported.

**Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles cleanly.

**Step 4: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "feat: wire binary source type into skill installer pipeline"
```

---

## Task 9: Add `--bin` flag to `ion add` command

**Files:**
- Modify: `src/main.rs:18-24` (add `bin` flag to `Add` command)
- Modify: `src/commands/add.rs` (handle `--bin` flag)

**Step 1: Add `--bin` flag to CLI**

In `src/main.rs`, modify the `Add` command variant (~line 18):

```rust
Add {
    /// Skill source (e.g., owner/repo, URL, or local path)
    source: String,
    /// Git revision (branch, tag, or commit)
    #[arg(long)]
    rev: Option<String>,
    /// Install as a binary CLI skill from GitHub Releases
    #[arg(long)]
    bin: bool,
},
```

Update the match arm in `main()` (~line 121) to pass `bin`:

```rust
Commands::Add { source, rev, bin } => commands::add::run(&source, rev.as_deref(), bin),
```

**Step 2: Update `add.rs` to accept and handle the `bin` flag**

Change function signature:

```rust
pub fn run(source_str: &str, rev: Option<&str>, bin: bool) -> anyhow::Result<()> {
```

After inferring the source (~line 16), if `bin` is true, override source type:

```rust
let mut source = SkillSource::infer(resolved)?;
if let Some(r) = rev {
    source.rev = Some(r.to_string());
}
if bin {
    source.source_type = SourceType::Binary;
    // If no explicit binary name, derive from source
    if source.binary.is_none() {
        source.binary = Some(skill_name_from_source(&source));
    }
}
```

The rest of the add flow works as-is because the installer now handles `SourceType::Binary`.

**Step 3: Verify compilation and test**

Run: `cargo build`
Run: `cargo run -- add --help` — verify `--bin` flag appears.

**Step 4: Commit**

```bash
git add src/main.rs src/commands/add.rs
git commit -m "feat: add --bin flag to ion add for binary CLI skills"
```

---

## Task 10: Implement `ion run` command

**Files:**
- Create: `src/commands/run.rs`
- Modify: `src/main.rs` (add `Run` command)
- Modify: `src/commands/mod.rs` (if it exists, add `pub mod run;`)

**Step 1: Check if commands/mod.rs exists**

Check whether `src/commands/mod.rs` exists or if commands are declared inline. Looking at `main.rs` imports to determine module structure.

**Step 2: Write the `ion run` command**

Create `src/commands/run.rs`:

```rust
use anyhow::bail;
use ion_skill::binary;
use ion_skill::lockfile::Lockfile;

pub fn run(name: &str, args: &[String]) -> anyhow::Result<()> {
    // Load lockfile to find binary info
    let lockfile_path = std::path::Path::new("Ion.lock");
    if !lockfile_path.exists() {
        bail!("No Ion.lock found. Run `ion install` first.");
    }

    let lockfile = Lockfile::from_file(lockfile_path)?;
    let locked = lockfile.find(name).ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' not found in lockfile. Run `ion add {} --bin` first.", name, name)
    })?;

    let binary_name = locked.binary.as_deref().ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' is not a binary skill (no binary field in lockfile).", name)
    })?;

    let version = locked.binary_version.as_deref().ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' has no binary_version in lockfile. Try `ion install`.", name)
    })?;

    let binary_path = binary::binary_path(binary_name, version);
    if !binary_path.exists() {
        bail!(
            "Binary '{}' v{} not found at {}. Run `ion install` to download it.",
            binary_name,
            version,
            binary_path.display()
        );
    }

    // Exec the binary, replacing this process
    let status = std::process::Command::new(&binary_path)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", binary_path.display(), e))?;

    std::process::exit(status.code().unwrap_or(1));
}
```

**Step 3: Add `Run` command to CLI**

In `src/main.rs`, add to the `Commands` enum:

```rust
/// Run a binary skill
Run {
    /// Name of the binary skill to run
    name: String,
    /// Arguments to pass to the binary
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
},
```

Add the match arm in `main()`:

```rust
Commands::Run { name, args } => commands::run::run(&name, &args),
```

**Step 4: Register the module**

If `src/commands/mod.rs` exists, add `pub mod run;`. If commands are imported directly in `main.rs`, add the appropriate `mod` declaration matching the existing pattern.

**Step 5: Verify compilation and test**

Run: `cargo build`
Run: `cargo run -- run --help` — verify help text appears.

**Step 6: Commit**

```bash
git add src/commands/run.rs src/main.rs
git commit -m "feat: add ion run command for executing binary skills"
```

---

## Task 11: Integration test — full binary skill lifecycle

**Files:**
- Create: `tests/binary_integration.rs`

**Step 1: Write an integration test**

This test creates a fake GitHub-like setup to test the manifest/lockfile roundtrip without real network calls. For the full pipeline we test the data model pieces:

```rust
use std::fs;
use tempfile::tempdir;

/// Test that binary skill entries roundtrip through Ion.toml correctly.
#[test]
fn test_binary_skill_manifest_roundtrip() {
    let toml_content = r#"
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }
brainstorming = "anthropics/skills/brainstorming"
"#;

    let manifest = ion_skill::manifest::Manifest::parse(toml_content).unwrap();
    assert_eq!(manifest.skills.len(), 2);

    let entry = manifest.skills.get("mytool").unwrap();
    let source = ion_skill::manifest::Manifest::resolve_entry(entry).unwrap();

    assert_eq!(source.source_type, ion_skill::source::SourceType::Binary);
    assert_eq!(source.source, "owner/mytool");
    assert_eq!(source.binary.as_deref(), Some("mytool"));
}

/// Test that LockedSkill with binary fields roundtrips through TOML.
#[test]
fn test_binary_locked_skill_roundtrip() {
    let locked = ion_skill::lockfile::LockedSkill {
        name: "mytool".to_string(),
        source: "https://github.com/owner/mytool.git".to_string(),
        path: None,
        version: Some("1.2.0".to_string()),
        commit: None,
        checksum: None,
        binary: Some("mytool".to_string()),
        binary_version: Some("1.2.0".to_string()),
        binary_checksum: Some("sha256:abc123".to_string()),
    };

    let lockfile = ion_skill::lockfile::Lockfile {
        skills: vec![locked],
    };

    let tmp = tempdir().unwrap();
    let path = tmp.path().join("Ion.lock");
    lockfile.write_to(&path).unwrap();

    let loaded = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    assert_eq!(loaded.skills.len(), 1);
    assert_eq!(loaded.skills[0].binary.as_deref(), Some("mytool"));
    assert_eq!(loaded.skills[0].binary_version.as_deref(), Some("1.2.0"));
    assert_eq!(
        loaded.skills[0].binary_checksum.as_deref(),
        Some("sha256:abc123")
    );
}

/// Test platform detection produces valid values.
#[test]
fn test_platform_detection_produces_valid_triple() {
    let platform = ion_skill::binary::Platform::detect();
    let triple = platform.target_triple();
    assert!(
        triple.contains("darwin") || triple.contains("linux") || triple.contains("windows"),
        "Unexpected triple: {}",
        triple
    );
}
```

**Step 2: Run integration tests**

Run: `cargo test --test binary_integration`
Expected: All 3 tests pass.

**Step 3: Commit**

```bash
git add tests/binary_integration.rs
git commit -m "test: add integration tests for binary skill data model roundtrip"
```

---

## Task 12: Wire `ion install` to handle binary skills from lockfile

**Files:**
- Modify: `src/commands/install.rs` (handle binary entries during install)

**Step 1: Review current install flow**

The install command iterates `manifest.skills`, resolves each to `SkillSource`, and calls `installer.install()`. Since we already wired `SourceType::Binary` into the installer (Task 8), the install command should already work — the manifest resolver passes through the binary field.

Verify by checking that `Manifest::resolve_entry()` correctly propagates `binary` for `SkillEntry::Full` with `type = "binary"`.

**Step 2: Test manually**

Run: `cargo build`

Create a test `Ion.toml` in a temp dir with a binary skill entry. Run `ion install` and verify it attempts the binary install path (will fail on network but should show the right error path).

**Step 3: If any fixes needed, apply and commit**

Run: `cargo test`
Expected: All tests pass.

```bash
git add -A
git commit -m "feat: ensure ion install handles binary skills from manifest"
```

---

## Summary

After completing all 12 tasks, Phase 1 delivers:

| Feature | Task(s) |
|---------|---------|
| `Binary` source type | 1, 2 |
| Platform detection | 3 |
| GitHub Releases API | 4 |
| Binary download/extract/storage | 5 |
| SKILL.md generation via `<binary> skill` | 6 |
| Bundled SKILL.md fallback | 7 |
| Installer integration | 8 |
| `ion add --bin` | 9 |
| `ion run` command | 10 |
| Lockfile binary fields | 2 |
| Integration tests | 11 |
| `ion install` binary support | 12 |

**Not included in Phase 1** (deferred to later phases):
- `ion update` for binaries (Phase 2)
- Binary cleanup on `ion remove` (Phase 2)
- Generic URL downloads (Phase 3)
- `ion new --bin` scaffolding (Phase 4)
