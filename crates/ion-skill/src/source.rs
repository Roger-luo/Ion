use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// The type of source a skill is fetched from.
///
/// Kept for Ion.toml serde deserialization (used by `SkillEntry::Full` in manifest.rs).
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

/// Per-source-type data that only makes sense for that variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSourceKind {
    Github,
    Git,
    Http,
    Path,
    Binary {
        binary_name: String,
        asset_pattern: Option<String>,
        /// If set, this is a local binary project (build from source).
        local_project: Option<std::path::PathBuf>,
        dev: bool,
    },
    Local {
        forked_from: Option<String>,
    },
}

/// A fully resolved skill source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    /// The raw source string (URL, path, or owner/repo shorthand).
    pub source: String,
    /// Subdirectory path within the source (for multi-skill repos).
    pub path: Option<String>,
    /// Pinned revision (git commit, tag, or branch).
    pub rev: Option<String>,
    /// Required SKILL.md version.
    pub version: Option<String>,
    /// Source-type-specific data.
    pub kind: SkillSourceKind,
}

impl SkillSource {
    // ── Named constructors ──────────────────────────────────────────

    pub fn github(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SkillSourceKind::Github,
            path: None,
            rev: None,
            version: None,
        }
    }

    pub fn git(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SkillSourceKind::Git,
            path: None,
            rev: None,
            version: None,
        }
    }

    pub fn http(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SkillSourceKind::Http,
            path: None,
            rev: None,
            version: None,
        }
    }

    pub fn path(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SkillSourceKind::Path,
            path: None,
            rev: None,
            version: None,
        }
    }

    pub fn local() -> Self {
        Self {
            source: String::new(),
            kind: SkillSourceKind::Local { forked_from: None },
            path: None,
            rev: None,
            version: None,
        }
    }

    pub fn binary(source: impl Into<String>, binary_name: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SkillSourceKind::Binary {
                binary_name: binary_name.into(),
                asset_pattern: None,
                local_project: None,
                dev: false,
            },
            path: None,
            rev: None,
            version: None,
        }
    }

    // ── Compatibility shim ──────────────────────────────────────────

    /// Compatibility: convert SourceType to SkillSourceKind.
    ///
    /// Used by `manifest.rs::SkillEntry::resolve()`. Will be removed once
    /// callers migrate to the named constructors.
    pub fn new(source_type: SourceType, source: impl Into<String>) -> Self {
        let source = source.into();
        let kind = match source_type {
            SourceType::Github => SkillSourceKind::Github,
            SourceType::Git => SkillSourceKind::Git,
            SourceType::Http => SkillSourceKind::Http,
            SourceType::Path => SkillSourceKind::Path,
            SourceType::Binary => SkillSourceKind::Binary {
                binary_name: String::new(),
                asset_pattern: None,
                local_project: None,
                dev: false,
            },
            SourceType::Local => SkillSourceKind::Local { forked_from: None },
        };
        Self {
            source,
            path: None,
            rev: None,
            version: None,
            kind,
        }
    }

    /// Create a path-based skill source.
    ///
    /// Alias for `Self::path(source)`. Kept for backward compatibility.
    pub fn from_path(source: &str) -> Self {
        Self::path(source)
    }

    // ── Builder methods ─────────────────────────────────────────────

    pub fn with_rev(mut self, rev: impl Into<String>) -> Self {
        self.rev = Some(rev.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        let binary_name = binary.into();
        match &mut self.kind {
            SkillSourceKind::Binary {
                binary_name: bn, ..
            } => {
                *bn = binary_name;
            }
            _ => {
                self.kind = SkillSourceKind::Binary {
                    binary_name,
                    asset_pattern: None,
                    local_project: None,
                    dev: false,
                };
            }
        }
        self
    }

    pub fn with_asset_pattern(mut self, pattern: impl Into<String>) -> Self {
        if let SkillSourceKind::Binary {
            ref mut asset_pattern,
            ..
        } = self.kind
        {
            *asset_pattern = Some(pattern.into());
        }
        self
    }

    pub fn with_forked_from(mut self, forked_from: impl Into<String>) -> Self {
        if let SkillSourceKind::Local {
            forked_from: ref mut ff,
        } = self.kind
        {
            *ff = Some(forked_from.into());
        }
        self
    }

    pub fn with_dev(mut self, dev_mode: bool) -> Self {
        if let SkillSourceKind::Binary { ref mut dev, .. } = self.kind {
            *dev = dev_mode;
        }
        self
    }

    // ── Convenience predicates ──────────────────────────────────────

    pub fn is_github(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Github)
    }

    pub fn is_git_based(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Github | SkillSourceKind::Git)
    }

    pub fn is_binary(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Binary { .. })
    }

    pub fn is_local(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Local { .. })
    }

    pub fn is_path(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Path)
    }

    pub fn is_http(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Http)
    }

    /// Returns true if this source points to a local filesystem path
    /// (either a Path source or a Binary source with a local project).
    pub fn is_local_path(&self) -> bool {
        matches!(self.kind, SkillSourceKind::Path)
            || matches!(
                self.kind,
                SkillSourceKind::Binary {
                    local_project: Some(_),
                    ..
                }
            )
    }

    /// True for sources that need gitignore entries (not Path or Local).
    pub fn is_remote_installable(&self) -> bool {
        !matches!(
            self.kind,
            SkillSourceKind::Path | SkillSourceKind::Local { .. }
        )
    }

    // ── Derived data ────────────────────────────────────────────────

    /// Derive a human-readable skill name from this source.
    /// Uses the path's last segment if available, otherwise the source's last segment.
    pub fn display_name(&self) -> String {
        if let Some(ref path) = self.path {
            return path.rsplit('/').next().unwrap_or(path).to_string();
        }

        if self.is_http() {
            // For HTTP sources, strip trailing /skill.md and use the last path segment.
            // e.g. "https://example.com/docs/skill.md" → "docs"
            // e.g. "https://example.com/docs" → "docs"
            let url = self.source.trim_end_matches('/');
            let path = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .unwrap_or(url);
            // Remove host
            let path = path.split_once('/').map(|(_, p)| p).unwrap_or(path);
            // Strip trailing skill.md
            let path = path
                .strip_suffix("/skill.md")
                .or_else(|| path.strip_suffix("/SKILL.md"))
                .unwrap_or(path);
            return path
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("skill")
                .to_string();
        }

        self.source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&self.source)
            .to_string()
    }

    /// Infer a SkillSource from a raw source string (no explicit type).
    pub fn infer(source: &str) -> Result<Self> {
        // Local paths
        if source.starts_with('/') || source.starts_with("./") || source.starts_with("../") {
            return Ok(Self::path(source));
        }

        // URLs
        if source.starts_with("https://") || source.starts_with("http://") {
            if source.contains("github.com") {
                return Ok(Self::github(source));
            } else if source.ends_with(".git") {
                return Ok(Self::git(source));
            } else {
                return Ok(Self::http(source));
            }
        }

        // Shorthand: owner/repo or owner/repo/skill-path
        let segments: Vec<&str> = source.split('/').collect();
        match segments.len() {
            2 => Ok(Self::github(source)),
            3.. => Ok(Self::github(format!("{}/{}", segments[0], segments[1]))
                .with_path(segments[2..].join("/"))),
            _ => Err(Error::Source(format!(
                "Cannot infer source type from: {source}"
            ))),
        }
    }

    /// Return the URL to fetch the SKILL.md file from an HTTP source.
    /// If the URL doesn't already end with `skill.md` (case-insensitive),
    /// appends `/skill.md`.
    pub fn http_skill_url(&self) -> Result<String> {
        if !self.is_http() {
            return Err(Error::Source(format!(
                "Source kind {:?} is not HTTP",
                self.kind
            )));
        }
        let url = self.source.trim_end_matches('/');
        if url
            .rsplit('/')
            .next()
            .is_some_and(|last| last.eq_ignore_ascii_case("skill.md"))
        {
            Ok(url.to_string())
        } else {
            Ok(format!("{url}/skill.md"))
        }
    }

    /// Build a git clone URL for this source.
    pub fn git_url(&self) -> Result<String> {
        match &self.kind {
            SkillSourceKind::Github => {
                if self.source.starts_with("https://") {
                    return Ok(self.source.clone());
                }
                Ok(format!("https://github.com/{}.git", self.source))
            }
            SkillSourceKind::Git => Ok(self.source.clone()),
            _ => Err(Error::Source(format!(
                "Source kind {:?} has no git URL",
                self.kind
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_github_three_segments() {
        let s = SkillSource::infer("obra/superpowers/brainstorming").unwrap();
        assert!(s.is_github());
        assert_eq!(s.source, "obra/superpowers");
        assert_eq!(s.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn infer_github_two_segments() {
        let s = SkillSource::infer("org/my-skill").unwrap();
        assert!(s.is_github());
        assert_eq!(s.source, "org/my-skill");
        assert_eq!(s.path, None);
    }

    #[test]
    fn infer_github_url() {
        let s = SkillSource::infer("https://github.com/org/repo.git").unwrap();
        assert!(s.is_github());
    }

    #[test]
    fn infer_git_url() {
        let s = SkillSource::infer("https://gitlab.com/org/repo.git").unwrap();
        assert!(matches!(s.kind, SkillSourceKind::Git));
    }

    #[test]
    fn infer_http_url() {
        let s = SkillSource::infer("https://example.com/skill.tar.gz").unwrap();
        assert!(s.is_http());
    }

    #[test]
    fn infer_local_relative_path() {
        let s = SkillSource::infer("../my-skill").unwrap();
        assert!(s.is_path());
    }

    #[test]
    fn infer_local_absolute_path() {
        let s = SkillSource::infer("/home/user/skills/my-skill").unwrap();
        assert!(s.is_path());
    }

    #[test]
    fn infer_local_current_dir_path() {
        let s = SkillSource::infer("./my-skill").unwrap();
        assert!(s.is_path());
    }

    #[test]
    fn infer_github_four_segments() {
        let s = SkillSource::infer("obra/superpowers/skills/brainstorming").unwrap();
        assert!(s.is_github());
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
    fn test_binary_source() {
        let source = SkillSource::new(SourceType::Binary, "owner/mytool").with_binary("mytool");
        assert!(source.is_binary());
        match &source.kind {
            SkillSourceKind::Binary { binary_name, .. } => {
                assert_eq!(binary_name, "mytool");
            }
            _ => panic!("Expected Binary kind"),
        }
    }

    #[test]
    fn test_binary_named_constructor() {
        let source = SkillSource::binary("owner/mytool", "mytool");
        assert!(source.is_binary());
        match &source.kind {
            SkillSourceKind::Binary { binary_name, .. } => {
                assert_eq!(binary_name, "mytool");
            }
            _ => panic!("Expected Binary kind"),
        }
    }

    #[test]
    fn http_skill_url_appends_skill_md() {
        let s = SkillSource::infer("https://www.mintlify.com/docs").unwrap();
        assert!(s.is_http());
        assert_eq!(
            s.http_skill_url().unwrap(),
            "https://www.mintlify.com/docs/skill.md"
        );
    }

    #[test]
    fn http_skill_url_preserves_existing_skill_md() {
        let s = SkillSource::infer("https://www.mintlify.com/docs/skill.md").unwrap();
        assert_eq!(
            s.http_skill_url().unwrap(),
            "https://www.mintlify.com/docs/skill.md"
        );
    }

    #[test]
    fn http_skill_url_case_insensitive() {
        let s = SkillSource::infer("https://example.com/skills/SKILL.md").unwrap();
        assert_eq!(
            s.http_skill_url().unwrap(),
            "https://example.com/skills/SKILL.md"
        );
    }

    #[test]
    fn http_skill_url_strips_trailing_slash() {
        let s = SkillSource::infer("https://www.mintlify.com/docs/").unwrap();
        assert_eq!(
            s.http_skill_url().unwrap(),
            "https://www.mintlify.com/docs/skill.md"
        );
    }

    #[test]
    fn http_display_name_from_url() {
        let s = SkillSource::infer("https://www.mintlify.com/docs").unwrap();
        assert_eq!(s.display_name(), "docs");
    }

    #[test]
    fn http_display_name_strips_skill_md() {
        let s = SkillSource::infer("https://www.mintlify.com/docs/skill.md").unwrap();
        assert_eq!(s.display_name(), "docs");
    }

    #[test]
    fn git_url_path_is_error() {
        let s = SkillSource::infer("./local").unwrap();
        assert!(s.git_url().is_err());
    }

    #[test]
    fn is_local_path_for_relative_path() {
        let s = SkillSource::infer("./my-skill").unwrap();
        assert!(s.is_local_path());
    }

    #[test]
    fn is_local_path_for_absolute_path() {
        let s = SkillSource::infer("/home/user/skill").unwrap();
        assert!(s.is_local_path());
    }

    #[test]
    fn is_local_path_false_for_github() {
        let s = SkillSource::infer("org/repo").unwrap();
        assert!(!s.is_local_path());
    }

    #[test]
    fn is_local_path_false_for_url() {
        let s = SkillSource::infer("https://github.com/org/repo.git").unwrap();
        assert!(!s.is_local_path());
    }

    #[test]
    fn is_local_path_for_parent_relative_path() {
        let s = SkillSource::infer("../my-skill").unwrap();
        assert!(s.is_local_path());
    }

    #[test]
    fn local_source() {
        let source = SkillSource::local().with_forked_from("org/original-skill");
        assert!(source.is_local());
        match &source.kind {
            SkillSourceKind::Local { forked_from } => {
                assert_eq!(forked_from.as_deref(), Some("org/original-skill"));
            }
            _ => panic!("Expected Local kind"),
        }
        assert!(source.git_url().is_err());
    }

    #[test]
    fn compatibility_shim_new() {
        let s = SkillSource::new(SourceType::Github, "org/repo");
        assert!(s.is_github());
        assert_eq!(s.source, "org/repo");

        let s = SkillSource::new(SourceType::Local, "");
        assert!(s.is_local());
    }

    #[test]
    fn named_constructors() {
        assert!(SkillSource::github("org/repo").is_github());
        assert!(SkillSource::git("https://gitlab.com/repo.git").is_git_based());
        assert!(SkillSource::http("https://example.com").is_http());
        assert!(SkillSource::path("./local").is_path());
        assert!(SkillSource::local().is_local());
        assert!(SkillSource::binary("owner/tool", "tool").is_binary());
    }

    #[test]
    fn is_remote_installable() {
        assert!(SkillSource::github("org/repo").is_remote_installable());
        assert!(SkillSource::git("https://gitlab.com/repo.git").is_remote_installable());
        assert!(SkillSource::http("https://example.com").is_remote_installable());
        assert!(SkillSource::binary("owner/tool", "tool").is_remote_installable());
        assert!(!SkillSource::path("./local").is_remote_installable());
        assert!(!SkillSource::local().is_remote_installable());
    }
}
