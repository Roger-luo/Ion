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
    Binary,
    Local,
}

/// A fully resolved skill source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub source_type: SourceType,
    pub source: String,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub binary: Option<String>,
    pub asset_pattern: Option<String>,
    pub forked_from: Option<String>,
}

impl SkillSource {
    /// Infer a SkillSource from a raw source string (no explicit type).
    pub fn infer(source: &str) -> Result<Self> {
        // Local paths
        if source.starts_with('/') || source.starts_with("./") || source.starts_with("../") {
            return Ok(Self {
                source_type: SourceType::Path,
                source: source.to_string(),
                path: None,
                rev: None,
                version: None,
                binary: None,
                asset_pattern: None,
                forked_from: None,
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
                binary: None,
                asset_pattern: None,
                forked_from: None,
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
                binary: None,
                asset_pattern: None,
                forked_from: None,
            }),
            3.. => Ok(Self {
                source_type: SourceType::Github,
                source: format!("{}/{}", segments[0], segments[1]),
                path: Some(segments[2..].join("/")),
                rev: None,
                version: None,
                binary: None,
                asset_pattern: None,
                forked_from: None,
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
                if self.source.starts_with("https://") {
                    return Ok(self.source.clone());
                }
                Ok(format!("https://github.com/{}.git", self.source))
            }
            SourceType::Git => Ok(self.source.clone()),
            _ => Err(Error::Source(format!(
                "Source type {:?} has no git URL",
                self.source_type
            ))),
        }
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
    }

    #[test]
    fn infer_git_url() {
        let s = SkillSource::infer("https://gitlab.com/org/repo.git").unwrap();
        assert_eq!(s.source_type, SourceType::Git);
    }

    #[test]
    fn infer_http_url() {
        let s = SkillSource::infer("https://example.com/skill.tar.gz").unwrap();
        assert_eq!(s.source_type, SourceType::Http);
    }

    #[test]
    fn infer_local_relative_path() {
        let s = SkillSource::infer("../my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
    }

    #[test]
    fn infer_local_absolute_path() {
        let s = SkillSource::infer("/home/user/skills/my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
    }

    #[test]
    fn infer_local_current_dir_path() {
        let s = SkillSource::infer("./my-skill").unwrap();
        assert_eq!(s.source_type, SourceType::Path);
    }

    #[test]
    fn infer_github_four_segments() {
        let s = SkillSource::infer("obra/superpowers/skills/brainstorming").unwrap();
        assert_eq!(s.source_type, SourceType::Github);
        assert_eq!(s.source, "obra/superpowers");
        assert_eq!(s.path.as_deref(), Some("skills/brainstorming"));
    }

    #[test]
    fn infer_single_segment_is_error() {
        let result = SkillSource::infer("brainstorming");
        assert!(result.is_err());
    }

    #[test]
    fn git_url_github_shorthand() {
        let s = SkillSource::infer("org/repo").unwrap();
        assert_eq!(s.git_url().unwrap(), "https://github.com/org/repo.git");
    }

    #[test]
    fn test_binary_source_type_serializes() {
        let source = SkillSource {
            source_type: SourceType::Binary,
            source: "owner/mytool".to_string(),
            path: None,
            rev: None,
            version: None,
            binary: Some("mytool".to_string()),
            asset_pattern: None,
            forked_from: None,
        };
        assert_eq!(source.source_type, SourceType::Binary);
        assert_eq!(source.binary.as_deref(), Some("mytool"));
    }

    #[test]
    fn git_url_path_is_error() {
        let s = SkillSource::infer("./local").unwrap();
        assert!(s.git_url().is_err());
    }

    #[test]
    fn local_source_type_serializes() {
        let source = SkillSource {
            source_type: SourceType::Local,
            source: "./my-skill".to_string(),
            path: None,
            rev: None,
            version: None,
            binary: None,
            asset_pattern: None,
            forked_from: Some("org/original-skill".to_string()),
        };
        assert_eq!(source.source_type, SourceType::Local);
        assert_eq!(source.forked_from.as_deref(), Some("org/original-skill"));
        assert!(source.git_url().is_err());
    }
}
