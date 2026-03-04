use std::path::PathBuf;

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

pub fn current_binary_path(name: &str) -> PathBuf {
    bin_dir().join(name).join("current").join(name)
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
}
