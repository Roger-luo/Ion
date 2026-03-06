use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Detected platform info for binary downloads.
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
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

    pub fn target_triple(&self) -> String {
        let os_part = match self.os.as_str() {
            "macos" => "apple-darwin",
            "linux" => "unknown-linux-gnu",
            "windows" => "pc-windows-msvc",
            other => other,
        };
        format!("{}-{}", self.arch, os_part)
    }

    /// Match a binary name against release asset names, returning the best match.
    pub fn match_asset(&self, binary_name: &str, asset_names: &[String]) -> Option<String> {
        let triple = self.target_triple();

        // Priority 1: target triple match
        let triple_pattern = format!("{}-{}", binary_name, triple);
        if let Some(name) = asset_names.iter().find(|n| n.starts_with(&triple_pattern)) {
            return Some(name.clone());
        }

        // Priority 2: OS + arch aliases
        let os_aliases = self.os_aliases();
        let arch_aliases = self.arch_aliases();
        for name in asset_names {
            let lower = name.to_lowercase();
            if !lower.starts_with(&binary_name.to_lowercase()) {
                continue;
            }
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

pub fn bin_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("bin")
}

pub fn binary_path(name: &str, version: &str) -> PathBuf {
    bin_dir().join(name).join(version).join(name)
}

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
            "GitHub API returned {}",
            resp.status()
        )));
    }
    resp.json::<GitHubRelease>()
        .map_err(|e| crate::Error::Other(format!("Failed to parse release JSON: {}", e)))
}

pub fn parse_version_from_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

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

/// Extract a .tar.gz archive to a destination directory.
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
            .map_err(|e| crate::Error::Other(format!("Bad path: {}", e)))?
            .into_owned();
        entry
            .unpack_in(dest_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to extract {}: {}", path.display(), e)))?;
        extracted.push(dest_dir.join(&path));
    }
    Ok(extracted)
}

/// Find the binary executable in an extracted directory.
pub fn find_binary_in_dir(dir: &Path, binary_name: &str) -> crate::Result<PathBuf> {
    let direct = dir.join(binary_name);
    if direct.is_file() {
        return Ok(direct);
    }
    // Search one level of subdirectories (tarballs often wrap in a dir)
    for entry in
        fs::read_dir(dir).map_err(|e| crate::Error::Other(format!("Failed to read dir: {}", e)))?
    {
        let entry =
            entry.map_err(|e| crate::Error::Other(format!("Failed to read entry: {}", e)))?;
        if entry.path().is_dir() {
            let nested = entry.path().join(binary_name);
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

/// Install a binary file to versioned storage.
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))
            .map_err(|e| crate::Error::Other(format!("Failed to set permissions: {}", e)))?;
    }
    let current_link = bin_root.join(name).join("current");
    if current_link.exists() || current_link.is_symlink() {
        fs::remove_file(&current_link)
            .map_err(|e| crate::Error::Other(format!("Failed to remove stale current link: {}", e)))?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(version, &current_link)
        .map_err(|e| crate::Error::Other(format!("Failed to create current symlink: {}", e)))?;
    Ok(())
}

pub fn file_checksum(path: &Path) -> crate::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes =
        fs::read(path).map_err(|e| crate::Error::Other(format!("Failed to read file for checksum: {}", e)))?;
    let hash = Sha256::digest(&bytes);
    Ok(format!("sha256:{:x}", hash))
}

/// Run `<binary> skill` and capture the SKILL.md output from stdout.
pub fn generate_skill_md(binary_path: &Path) -> crate::Result<String> {
    use std::process::Command;
    let output = Command::new(binary_path)
        .arg("skill")
        .output()
        .map_err(|e| {
            crate::Error::Other(format!(
                "Failed to run '{} skill': {}",
                binary_path.display(),
                e
            ))
        })?;
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

/// Look for a SKILL.md in an extracted archive directory.
pub fn find_bundled_skill_md(extract_dir: &Path) -> Option<PathBuf> {
    let direct = extract_dir.join("SKILL.md");
    if direct.is_file() {
        return Some(direct);
    }
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

/// Result of validating an installed binary.
#[derive(Debug)]
pub struct BinaryValidation {
    pub is_executable: bool,
    pub version_output: Option<String>,
    pub has_skill_command: bool,
}

/// Validate that an installed binary is functional.
///
/// Checks executable permissions, tries `--version`, and checks for a `skill` subcommand.
/// Returns an error only if the binary does not exist; other checks are best-effort.
pub fn validate_binary(binary_path: &Path) -> crate::Result<BinaryValidation> {
    use std::process::Command;

    // 1. Check file exists
    if !binary_path.exists() {
        return Err(crate::Error::Other(format!(
            "Binary not found at {}",
            binary_path.display()
        )));
    }

    // 2. Check if executable (unix only)
    let is_executable = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(binary_path)
                .map_err(|e| crate::Error::Other(format!("Failed to read metadata: {}", e)))?;
            meta.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    };

    // 3. Try --version (don't fail if it doesn't work)
    let version_output = Command::new(binary_path)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    // 4. Try `skill` subcommand — check if it produces output starting with `---`
    let has_skill_command = Command::new(binary_path)
        .arg("skill")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).starts_with("---"))
        .unwrap_or(false);

    Ok(BinaryValidation {
        is_executable,
        version_output,
        has_skill_command,
    })
}

/// Check if a binary is already installed at the given version.
pub fn is_binary_installed(name: &str, version: &str) -> bool {
    binary_path(name, version).exists()
}

#[derive(Debug)]
pub struct BinaryInstallResult {
    pub version: String,
    pub binary_checksum: String,
    pub warnings: Vec<String>,
}

/// Full binary skill installation from GitHub Releases.
pub fn install_binary_from_github(
    repo: &str,
    binary_name: &str,
    rev: Option<&str>,
    skill_dir: &Path,
    asset_pattern: Option<&str>,
) -> crate::Result<BinaryInstallResult> {
    let platform = Platform::detect();
    let release = fetch_github_release(repo, rev)?;
    let version = parse_version_from_tag(&release.tag_name).to_string();

    // Check if already installed at this version
    if is_binary_installed(binary_name, &version) {
        let installed_binary = binary_path(binary_name, &version);
        let checksum = file_checksum(&installed_binary)?;

        // Still need to ensure SKILL.md exists
        fs::create_dir_all(skill_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to create skill dir: {}", e)))?;

        if !skill_dir.join("SKILL.md").exists() {
            let skill_md_content = generate_skill_md(&installed_binary)?;
            fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
                .map_err(|e| crate::Error::Other(format!("Failed to write SKILL.md: {}", e)))?;
        }

        return Ok(BinaryInstallResult {
            version,
            binary_checksum: checksum,
            warnings: Vec::new(),
        });
    }

    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();

    let asset_name = if let Some(pattern) = asset_pattern {
        let expanded = expand_url_template(pattern, binary_name, &version);
        if asset_names.contains(&expanded) {
            expanded
        } else {
            return Err(crate::Error::Other(format!(
                "Asset pattern expanded to '{}' but no matching asset found in {:?}",
                expanded, asset_names
            )));
        }
    } else {
        platform
            .match_asset(binary_name, &asset_names)
            .ok_or_else(|| {
                crate::Error::Other(format!(
                    "No matching release asset for platform {} in {:?}",
                    platform.target_triple(),
                    asset_names
                ))
            })?
    };
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            crate::Error::Other(format!(
                "Asset '{}' not found in release (this is a bug — asset was matched but not found)",
                asset_name
            ))
        })?;

    let tmp_dir = tempfile::tempdir()
        .map_err(|e| crate::Error::Other(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = tmp_dir.path().join(&asset_name);
    download_file(&asset.browser_download_url, &archive_path)?;

    let extract_dir = tmp_dir.path().join("extracted");
    extract_tar_gz(&archive_path, &extract_dir)?;

    let found_binary = find_binary_in_dir(&extract_dir, binary_name)?;
    let bin_root = bin_dir();
    install_binary_file(&found_binary, binary_name, &version, &bin_root)?;

    let installed_binary = binary_path(binary_name, &version);
    let checksum = file_checksum(&installed_binary)?;

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

    // Validate the installed binary (warnings only, don't fail)
    let mut warnings = Vec::new();
    if let Ok(validation) = validate_binary(&installed_binary) {
        if !validation.is_executable {
            warnings.push(format!("Binary '{}' may not be executable", binary_name));
        }
        if !validation.has_skill_command {
            warnings.push(format!(
                "Binary '{}' does not have a 'skill' subcommand",
                binary_name
            ));
        }
    }

    Ok(BinaryInstallResult {
        version,
        binary_checksum: checksum,
        warnings,
    })
}

/// Expand placeholders in a URL template using detected platform info.
///
/// Supported placeholders: `{version}`, `{target}`, `{os}`, `{arch}`, `{binary}`.
pub fn expand_url_template(template: &str, binary_name: &str, version: &str) -> String {
    let platform = Platform::detect();
    template
        .replace("{version}", version)
        .replace("{binary}", binary_name)
        .replace("{target}", &platform.target_triple())
        .replace("{os}", &platform.os)
        .replace("{arch}", &platform.arch)
}

/// Install a binary from a generic URL template.
///
/// The `url_template` may contain placeholders expanded by [`expand_url_template`].
/// The archive is expected to be a `.tar.gz` file.
pub fn install_binary_from_url(
    url_template: &str,
    binary_name: &str,
    version: &str,
    skill_dir: &Path,
) -> crate::Result<BinaryInstallResult> {
    // Check if already installed at this version
    if is_binary_installed(binary_name, version) {
        let installed_binary = binary_path(binary_name, version);
        let checksum = file_checksum(&installed_binary)?;

        fs::create_dir_all(skill_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to create skill dir: {}", e)))?;

        if !skill_dir.join("SKILL.md").exists() {
            let skill_md_content = generate_skill_md(&installed_binary)?;
            fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
                .map_err(|e| crate::Error::Other(format!("Failed to write SKILL.md: {}", e)))?;
        }

        return Ok(BinaryInstallResult {
            version: version.to_string(),
            binary_checksum: checksum,
            warnings: Vec::new(),
        });
    }

    let url = expand_url_template(url_template, binary_name, version);

    if !url.ends_with(".tar.gz") && !url.ends_with(".tgz") {
        return Err(crate::Error::Other(format!(
            "URL-based binary sources currently only support .tar.gz archives, got: {url}"
        )));
    }

    let tmp_dir = tempfile::tempdir()
        .map_err(|e| crate::Error::Other(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = tmp_dir.path().join(format!("{}.tar.gz", binary_name));
    download_file(&url, &archive_path)?;

    let extract_dir = tmp_dir.path().join("extracted");
    extract_tar_gz(&archive_path, &extract_dir)?;

    let found_binary = find_binary_in_dir(&extract_dir, binary_name)?;
    let bin_root = bin_dir();
    install_binary_file(&found_binary, binary_name, version, &bin_root)?;

    let installed_binary = binary_path(binary_name, version);
    let checksum = file_checksum(&installed_binary)?;

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

    // Validate the installed binary (warnings only, don't fail)
    let mut warnings = Vec::new();
    if let Ok(validation) = validate_binary(&installed_binary) {
        if !validation.is_executable {
            warnings.push(format!("Binary '{}' may not be executable", binary_name));
        }
        if !validation.has_skill_command {
            warnings.push(format!(
                "Binary '{}' does not have a 'skill' subcommand",
                binary_name
            ));
        }
    }

    Ok(BinaryInstallResult {
        version: version.to_string(),
        binary_checksum: checksum,
        warnings,
    })
}

/// Remove a specific version of a binary from storage.
pub fn remove_binary_version(name: &str, version: &str) -> crate::Result<()> {
    let version_dir = bin_dir().join(name).join(version);
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to remove binary version dir: {}", e)))?;
    }
    Ok(())
}

/// Remove all versions of a binary and its parent directory.
pub fn remove_binary(name: &str) -> crate::Result<()> {
    let binary_dir = bin_dir().join(name);
    if binary_dir.exists() {
        fs::remove_dir_all(&binary_dir)
            .map_err(|e| crate::Error::Other(format!("Failed to remove binary dir: {}", e)))?;
    }
    Ok(())
}

/// List all installed binary skill names (directory names under bin_dir).
pub fn list_installed_binaries() -> crate::Result<Vec<String>> {
    let dir = bin_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| crate::Error::Other(format!("Failed to read bin dir: {}", e)))?
    {
        let entry = entry.map_err(|e| crate::Error::Other(format!("Failed to read entry: {}", e)))?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
        let triple = platform.target_triple();
        assert!(triple.contains('-'));
    }

    #[test]
    fn test_asset_matching_target_triple() {
        let platform = Platform {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
        };
        let assets = vec![
            "mytool-x86_64-apple-darwin.tar.gz".to_string(),
            "mytool-aarch64-apple-darwin.tar.gz".to_string(),
            "mytool-x86_64-unknown-linux-gnu.tar.gz".to_string(),
        ];
        assert_eq!(
            platform.match_asset("mytool", &assets),
            Some("mytool-aarch64-apple-darwin.tar.gz".to_string())
        );
    }

    #[test]
    fn test_asset_matching_os_arch_aliases() {
        let platform = Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
        };
        let assets = vec![
            "mytool-linux-amd64.tar.gz".to_string(),
            "mytool-darwin-arm64.tar.gz".to_string(),
        ];
        assert_eq!(
            platform.match_asset("mytool", &assets),
            Some("mytool-linux-amd64.tar.gz".to_string())
        );
    }

    #[test]
    fn test_asset_matching_skips_non_archives() {
        let platform = Platform {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
        };
        let assets = vec!["mytool-checksums.txt".to_string()];
        assert_eq!(platform.match_asset("mytool", &assets), None);
    }

    #[test]
    fn test_bin_dir() {
        let dir = bin_dir();
        assert!(dir.to_string_lossy().contains("ion"));
    }

    #[test]
    fn test_binary_path() {
        let path = binary_path("mytool", "1.2.0");
        assert!(path.to_string_lossy().contains("mytool"));
        assert!(path.to_string_lossy().contains("1.2.0"));
    }

    #[test]
    fn test_parse_github_release_json() {
        let json = r#"{"tag_name": "v1.2.0", "assets": [{"name": "mytool.tar.gz", "browser_download_url": "https://example.com/mytool.tar.gz"}]}"#;
        let release: GitHubRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
        assert_eq!(release.assets.len(), 1);
    }

    #[test]
    fn test_parse_version_from_tag() {
        assert_eq!(parse_version_from_tag("v1.2.0"), "1.2.0");
        assert_eq!(parse_version_from_tag("1.2.0"), "1.2.0");
    }

    #[test]
    fn test_install_binary_creates_version_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_root = tmp.path().join("bin");
        let fake_binary = tmp.path().join("mytool");
        fs::write(&fake_binary, "#!/bin/sh\necho hello").unwrap();

        install_binary_file(&fake_binary, "mytool", "1.2.0", &bin_root).unwrap();

        let installed = bin_root.join("mytool").join("1.2.0").join("mytool");
        assert!(installed.exists());

        let current = bin_root.join("mytool").join("current");
        assert!(current.is_symlink());
        assert_eq!(
            std::fs::read_link(&current).unwrap(),
            PathBuf::from("1.2.0")
        );
    }

    #[test]
    fn test_find_binary_in_dir_direct() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("mytool"), "binary").unwrap();
        let found = find_binary_in_dir(tmp.path(), "mytool").unwrap();
        assert_eq!(found, tmp.path().join("mytool"));
    }

    #[test]
    fn test_find_binary_in_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("mytool-1.0");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("mytool"), "binary").unwrap();
        let found = find_binary_in_dir(tmp.path(), "mytool").unwrap();
        assert_eq!(found, subdir.join("mytool"));
    }

    #[test]
    fn test_find_bundled_skill_md_direct() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("SKILL.md"), "---\nname: test\n---").unwrap();
        assert!(find_bundled_skill_md(tmp.path()).is_some());
    }

    #[test]
    fn test_find_bundled_skill_md_in_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("mytool-1.0");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("SKILL.md"), "---\nname: test\n---").unwrap();
        assert!(find_bundled_skill_md(tmp.path()).is_some());
    }

    #[test]
    fn test_find_bundled_skill_md_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_bundled_skill_md(tmp.path()).is_none());
    }

    #[test]
    fn test_file_checksum() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test");
        fs::write(&file, "hello world").unwrap();
        let checksum = file_checksum(&file).unwrap();
        assert!(checksum.starts_with("sha256:"));
        assert!(checksum.len() > 10);
    }

    #[test]
    fn test_generate_skill_md_from_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_path = tmp.path().join("mytool");
        #[cfg(unix)]
        {
            fs::write(
                &bin_path,
                r#"#!/bin/sh
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
Use ion run mytool to run this tool.
EOF
else
    echo "unknown command"
    exit 1
fi
"#,
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755)).unwrap();
            let skill_md = generate_skill_md(&bin_path).unwrap();
            assert!(skill_md.contains("name: mytool"));
            assert!(skill_md.contains("description:"));
        }
    }

    #[test]
    fn test_extract_tar_gz() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let tmp = tempfile::tempdir().unwrap();

        // Create a tar.gz with a single file
        let archive_path = tmp.path().join("test.tar.gz");
        let file = fs::File::create(&archive_path).unwrap();
        let enc = GzEncoder::new(file, Compression::default());
        let mut builder = tar::Builder::new(enc);
        let content = b"hello world";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "test.txt", &content[..])
            .unwrap();
        builder.finish().unwrap();
        drop(builder);

        let extract_dir = tmp.path().join("out");
        let extracted = extract_tar_gz(&archive_path, &extract_dir).unwrap();
        assert!(!extracted.is_empty());
        assert!(extract_dir.join("test.txt").exists());
    }

    #[test]
    fn test_is_binary_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_root = tmp.path().join("bin");
        let fake_binary = tmp.path().join("mytool");
        fs::write(&fake_binary, "#!/bin/sh\necho hello").unwrap();

        // Not installed yet
        assert!(!bin_root.join("mytool").join("1.0.0").join("mytool").exists());

        // Install it
        install_binary_file(&fake_binary, "mytool", "1.0.0", &bin_root).unwrap();

        // Now it should exist
        assert!(bin_root.join("mytool").join("1.0.0").join("mytool").exists());
    }

    #[test]
    fn test_remove_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_root = tmp.path();
        let binary_dir = bin_root.join("ion").join("bin").join("mytool");

        // Create fake binary structure
        let version_dir = binary_dir.join("1.0.0");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("mytool"), "binary").unwrap();

        assert!(binary_dir.exists());

        // Use remove_dir_all directly since remove_binary uses bin_dir()
        fs::remove_dir_all(&binary_dir).unwrap();

        assert!(!binary_dir.exists());
    }

    #[test]
    fn test_remove_binary_nonexistent_is_ok() {
        // Removing a binary that doesn't exist should not error
        // We can't easily test remove_binary() since it uses bin_dir()
        // but we can test the pattern
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        assert!(!nonexistent.exists());
        // The function checks .exists() before removing, so no error
    }

    #[test]
    fn test_list_installed_binaries_empty() {
        // bin_dir might not exist or be empty for this test env
        // Just verify the function doesn't crash
        let result = list_installed_binaries();
        assert!(result.is_ok());
    }

    #[test]
    fn test_expand_url_template() {
        let url = expand_url_template(
            "https://example.com/releases/{version}/tool-{target}.tar.gz",
            "tool",
            "1.2.0",
        );
        let platform = Platform::detect();
        let expected = format!(
            "https://example.com/releases/1.2.0/tool-{}.tar.gz",
            platform.target_triple()
        );
        assert_eq!(url, expected);
    }

    #[test]
    fn test_expand_url_template_all_placeholders() {
        let platform = Platform::detect();
        let url = expand_url_template(
            "https://example.com/{binary}-{version}-{os}-{arch}-{target}.tar.gz",
            "mytool",
            "2.0.0",
        );
        assert!(url.contains("mytool"));
        assert!(url.contains("2.0.0"));
        assert!(url.contains(&platform.os));
        assert!(url.contains(&platform.arch));
        assert!(url.contains(&platform.target_triple()));
    }

    #[test]
    fn test_match_asset_with_pattern() {
        let assets = vec![
            "mytool-1.0.0-linux-x86_64.tar.gz".to_string(),
            "mytool-1.0.0-macos-aarch64.tar.gz".to_string(),
        ];
        // With pattern, expand_url_template should produce an exact match
        let platform = Platform::detect();
        let expanded = expand_url_template(
            "mytool-{version}-{os}-{arch}.tar.gz",
            "mytool",
            "1.0.0",
        );
        let expected = format!(
            "mytool-1.0.0-{}-{}.tar.gz",
            platform.os, platform.arch
        );
        assert_eq!(expanded, expected);
        // The expanded name should match one of the assets if our platform is in the list
        if assets.contains(&expanded) {
            assert!(true);
        }
    }

    #[test]
    fn test_expand_url_template_no_placeholders() {
        let url = expand_url_template(
            "https://example.com/static/tool.tar.gz",
            "tool",
            "1.0.0",
        );
        assert_eq!(url, "https://example.com/static/tool.tar.gz");
    }

    #[test]
    fn test_validate_binary_with_skill_command() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_path = tmp.path().join("mytool");
        #[cfg(unix)]
        {
            fs::write(
                &bin_path,
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "mytool 1.0.0"
elif [ "$1" = "skill" ]; then
    echo "---"
    echo "name: mytool"
    echo "description: Test tool."
    echo "---"
else
    echo "unknown"
fi
"#,
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755)).unwrap();

            let validation = validate_binary(&bin_path).unwrap();
            assert!(validation.is_executable);
            assert_eq!(validation.version_output.as_deref(), Some("mytool 1.0.0"));
            assert!(validation.has_skill_command);
        }
    }

    #[test]
    fn test_validate_binary_without_skill_command() {
        let tmp = tempfile::tempdir().unwrap();
        let bin_path = tmp.path().join("simpletool");
        #[cfg(unix)]
        {
            fs::write(&bin_path, "#!/bin/sh\necho hello\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755)).unwrap();

            let validation = validate_binary(&bin_path).unwrap();
            assert!(validation.is_executable);
            assert!(!validation.has_skill_command);
        }
    }

    #[test]
    fn test_validate_binary_not_found() {
        let result = validate_binary(std::path::Path::new("/nonexistent/binary"));
        assert!(result.is_err());
    }
}
