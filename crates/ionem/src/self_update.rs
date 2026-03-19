//! Reusable self-management infrastructure for binary skills.
//!
//! Binary skills built for the Ion ecosystem are expected to implement a standard
//! `self` subcommand group:
//!
//! - `<binary> self skill`   — print the embedded SKILL.md to stdout
//! - `<binary> self info`    — show version, build target, and executable path
//! - `<binary> self check`   — check if a newer version is available
//! - `<binary> self update`  — download and install a newer version
//!
//! This module provides [`SelfManager`] which implements the core logic for
//! `info`, `check`, and `update`. Downstream binary skills configure it with
//! their GitHub repo, binary name, and tag prefix, then delegate their `self`
//! subcommands to it.
//!
//! # Example
//!
//! ```rust,ignore
//! use ionem::self_update::SelfManager;
//!
//! let manager = SelfManager::new(
//!     "owner/my-tool",          // GitHub repo
//!     "my-tool",                // binary name in release assets
//!     "v",                      // tag prefix (e.g. "v1.0.0")
//!     env!("CARGO_PKG_VERSION"),
//!     env!("TARGET"),
//! );
//!
//! // In your clap match:
//! // SelfCommands::Skill => print!(include_str!("../SKILL.md")),
//! // SelfCommands::Info  => manager.print_info(),
//! // SelfCommands::Check => manager.print_check()?,
//! // SelfCommands::Update { version } => manager.run_update(version.as_deref())?,
//! ```

use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::release;

/// Configuration and executor for the standard binary skill `self` subcommands.
pub struct SelfManager {
    /// GitHub repository in `owner/repo` format.
    pub repo: String,
    /// The binary executable name (used for asset matching).
    pub binary_name: String,
    /// Tag prefix for release tags (e.g. `"v"` for `v1.0.0`, `"my-tool-v"` for `my-tool-v1.0.0`).
    pub tag_prefix: String,
    /// Current version of this binary (typically `env!("CARGO_PKG_VERSION")`).
    pub current_version: String,
    /// Build target triple (typically `env!("TARGET")`, set via `build.rs`).
    pub target: String,
}

/// Result of checking for updates.
#[derive(Debug)]
pub struct CheckResult {
    pub installed: String,
    pub latest: String,
    pub update_available: bool,
}

/// Result of performing an update.
#[derive(Debug)]
pub struct UpdateResult {
    pub updated: bool,
    pub old_version: String,
    pub new_version: String,
    pub exe: PathBuf,
}

/// Information about the current binary.
#[derive(Debug)]
pub struct SelfInfo {
    pub version: String,
    pub target: String,
    pub exe: PathBuf,
}

impl SelfManager {
    /// Create a new `SelfManager` with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `repo` — GitHub repository in `owner/repo` format
    /// * `binary_name` — executable name in release assets
    /// * `tag_prefix` — prefix before the version in git tags (e.g. `"v"`, `"my-tool-v"`)
    /// * `current_version` — the running binary's version (e.g. `env!("CARGO_PKG_VERSION")`)
    /// * `target` — build target triple (e.g. `env!("TARGET")`)
    pub fn new(
        repo: &str,
        binary_name: &str,
        tag_prefix: &str,
        current_version: &str,
        target: &str,
    ) -> Self {
        Self {
            repo: repo.to_string(),
            binary_name: binary_name.to_string(),
            tag_prefix: tag_prefix.to_string(),
            current_version: current_version.to_string(),
            target: target.to_string(),
        }
    }

    /// Return information about the current binary.
    pub fn info(&self) -> SelfInfo {
        SelfInfo {
            version: self.current_version.clone(),
            target: self.target.clone(),
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown")),
        }
    }

    /// Print self info to stdout.
    pub fn print_info(&self) {
        let info = self.info();
        println!("{} {}", self.binary_name, info.version);
        println!("target: {}", info.target);
        println!("exe: {}", info.exe.display());
    }

    /// Check whether a newer version is available on GitHub Releases.
    pub fn check(&self) -> Result<CheckResult> {
        let rel =
            release::fetch_latest_release_by_tag_prefix(&self.repo, &self.tag_prefix)?;
        let latest = release::parse_version_from_tag(&rel.tag_name).to_string();
        let update_available = is_newer_version(&self.current_version, &latest);

        Ok(CheckResult {
            installed: self.current_version.clone(),
            latest,
            update_available,
        })
    }

    /// Print check result to stdout.
    pub fn print_check(&self) -> Result<()> {
        let result = self.check()?;
        println!("installed: {}", result.installed);
        println!("latest:    {}", result.latest);

        if result.update_available {
            println!(
                "\nUpdate available: {} -> {}",
                result.installed, result.latest
            );
            println!(
                "Run `{} self update` to install it.",
                self.binary_name
            );
        } else {
            println!("\nAlready up to date.");
        }
        Ok(())
    }

    /// Download and install a newer version, replacing the current executable.
    ///
    /// If `version` is `None`, fetches the latest release. If `version` is `Some`,
    /// fetches the release tagged `{tag_prefix}{version}`.
    pub fn update(&self, version: Option<&str>) -> Result<UpdateResult> {
        let rel = match version {
            Some(v) => {
                let ver = v.strip_prefix('v').unwrap_or(v);
                let tag = format!("{}{}", self.tag_prefix, ver);
                release::fetch_github_release(&self.repo, Some(&tag))?
            }
            None => {
                release::fetch_latest_release_by_tag_prefix(&self.repo, &self.tag_prefix)?
            }
        };
        let latest = release::parse_version_from_tag(&rel.tag_name).to_string();

        if version.is_none() && !is_newer_version(&self.current_version, &latest) {
            return Ok(UpdateResult {
                updated: false,
                old_version: self.current_version.clone(),
                new_version: latest,
                exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown")),
            });
        }

        let platform = release::Platform::detect();
        let asset_names: Vec<String> = rel.assets.iter().map(|a| a.name.clone()).collect();

        let asset_name =
            platform
                .match_asset(&self.binary_name, &asset_names)
                .ok_or_else(|| {
                    Error::Other(format!(
                        "No prebuilt binary found for {}. Available assets: {}",
                        platform.target_triple(),
                        asset_names.join(", ")
                    ))
                })?;

        let asset = rel
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .expect("matched asset must exist in release");

        let tmp_dir = tempfile::tempdir()?;
        let archive_path = tmp_dir.path().join(&asset_name);
        release::download_file(&asset.browser_download_url, &archive_path)?;

        let extract_dir = tmp_dir.path().join("extracted");
        release::extract_tar_gz(&archive_path, &extract_dir)?;

        let new_binary = release::find_binary_in_dir(&extract_dir, &self.binary_name)?;
        let installed_path = replace_exe(&new_binary)?;

        Ok(UpdateResult {
            updated: true,
            old_version: self.current_version.clone(),
            new_version: latest,
            exe: installed_path,
        })
    }

    /// Run update and print progress to stdout.
    pub fn run_update(&self, version: Option<&str>) -> Result<()> {
        let result = self.update(version)?;

        if !result.updated {
            println!("Already up to date ({}).", result.old_version);
            return Ok(());
        }

        println!(
            "Updated {} {} -> {}",
            self.binary_name, result.old_version, result.new_version
        );
        println!("exe: {}", result.exe.display());
        Ok(())
    }
}

/// Replace the current running executable with a new binary.
///
/// Uses a backup-copy-cleanup strategy for atomic replacement.
pub fn replace_exe(new_binary: &std::path::Path) -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?
        .canonicalize()?;
    let backup = current_exe.with_extension("old");

    // Move current executable to backup
    if let Err(e) = std::fs::rename(&current_exe, &backup) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            return Err(Error::Other(format!(
                "Permission denied. Try: sudo {} self update",
                current_exe.display()
            )));
        }
        return Err(Error::Other(format!(
            "Failed to back up current executable: {}",
            e
        )));
    }

    // Copy new binary into place
    if let Err(e) = std::fs::copy(new_binary, &current_exe) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        return Err(Error::Other(format!(
            "Failed to install new binary: {}",
            e
        )));
    }

    // Set executable permissions on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Clean up backup
    let _ = std::fs::remove_file(&backup);

    Ok(current_exe)
}

/// Parse a version string like "0.1.14" into a comparable tuple.
fn parse_version_tuple(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Returns true if `latest` is strictly newer than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_version_tuple(current), parse_version_tuple(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => current != latest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.0", "0.1.1"));
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "1.0.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.2.0", "0.1.0"));
    }

    #[test]
    fn test_self_manager_info() {
        let manager = SelfManager::new("owner/repo", "mytool", "v", "1.0.0", "aarch64-apple-darwin");
        let info = manager.info();
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.target, "aarch64-apple-darwin");
    }
}
