//! GitHub release fetching and platform detection for binary skills.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// Detected platform info for binary downloads.
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
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

pub fn fetch_github_release(repo: &str, tag: Option<&str>) -> Result<GitHubRelease> {
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
        .map_err(|e| Error::Http(format!("Failed to fetch release: {}", e)))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!(
            "GitHub API returned {}",
            resp.status()
        )));
    }
    resp.json::<GitHubRelease>()
        .map_err(|e| Error::Http(format!("Failed to parse release JSON: {}", e)))
}

/// Fetch the latest release whose tag starts with the given prefix.
///
/// Useful when a repo has multiple crates releasing independently
/// (e.g. `ion-v*` vs `ion-skill-v*`).
pub fn fetch_latest_release_by_tag_prefix(repo: &str, prefix: &str) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases?per_page=10", repo);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "ion-skill-manager")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| Error::Http(format!("Failed to fetch releases: {}", e)))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!(
            "GitHub API returned {}",
            resp.status()
        )));
    }
    let releases: Vec<GitHubRelease> = resp
        .json()
        .map_err(|e| Error::Http(format!("Failed to parse releases JSON: {}", e)))?;
    releases
        .into_iter()
        .find(|r| r.tag_name.starts_with(prefix) && !r.assets.is_empty())
        .ok_or_else(|| Error::Other(format!("No release found with tag prefix '{}'", prefix)))
}

pub fn parse_version_from_tag(tag: &str) -> &str {
    // Handle release-plz style tags like "ion-v0.1.1" or "ion-skill-v0.1.0"
    if let Some(pos) = tag.rfind("-v") {
        &tag[pos + 2..]
    } else {
        tag.strip_prefix('v').unwrap_or(tag)
    }
}

/// Download a file from URL to a local path.
pub fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "ion-skill-manager")
        .send()
        .map_err(|e| Error::Http(format!("Download failed: {}", e)))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!("Download returned {}", resp.status())));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| Error::Http(format!("Failed to read response: {}", e)))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dest, &bytes)?;
    Ok(())
}

/// Extract a .tar.gz archive to a destination directory.
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    fs::create_dir_all(dest_dir)?;
    let mut extracted = Vec::new();
    for entry in archive
        .entries()
        .map_err(|e| Error::Other(format!("Failed to read archive: {}", e)))?
    {
        let mut entry = entry.map_err(|e| Error::Other(format!("Bad archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::Other(format!("Bad path: {}", e)))?
            .into_owned();
        entry
            .unpack_in(dest_dir)
            .map_err(|e| Error::Other(format!("Failed to extract {}: {}", path.display(), e)))?;
        extracted.push(dest_dir.join(&path));
    }
    Ok(extracted)
}

/// Find the binary executable in an extracted directory.
pub fn find_binary_in_dir(dir: &Path, binary_name: &str) -> Result<PathBuf> {
    let direct = dir.join(binary_name);
    if direct.is_file() {
        return Ok(direct);
    }
    // Search one level of subdirectories (tarballs often wrap in a dir)
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let nested = entry.path().join(binary_name);
            if nested.is_file() {
                return Ok(nested);
            }
        }
    }
    Err(Error::Other(format!(
        "Could not find binary '{}' in {}",
        binary_name,
        dir.display()
    )))
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
    fn test_parse_version_from_tag() {
        assert_eq!(parse_version_from_tag("v1.2.0"), "1.2.0");
        assert_eq!(parse_version_from_tag("1.2.0"), "1.2.0");
        assert_eq!(parse_version_from_tag("ion-v0.1.1"), "0.1.1");
        assert_eq!(parse_version_from_tag("ion-skill-v0.1.0"), "0.1.0");
    }

    #[test]
    fn test_parse_github_release_json() {
        let json = r#"{"tag_name": "v1.2.0", "assets": [{"name": "mytool.tar.gz", "browser_download_url": "https://example.com/mytool.tar.gz"}]}"#;
        let release: GitHubRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
        assert_eq!(release.assets.len(), 1);
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
    fn test_extract_tar_gz() {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let tmp = tempfile::tempdir().unwrap();

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
}
