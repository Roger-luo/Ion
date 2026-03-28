use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockedSkillKind {
    Git {
        commit: String,
        checksum: String,
    },
    Binary {
        binary_name: String,
        binary_version: Option<String>,
        binary_checksum: Option<String>,
        dev: bool,
    },
    Local {
        checksum: Option<String>,
    },
    Http {
        checksum: Option<String>,
    },
    Path {
        checksum: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSkill {
    pub name: String,
    pub source: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub kind: LockedSkillKind,
}

// ---------------------------------------------------------------------------
// Builder methods
// ---------------------------------------------------------------------------

impl LockedSkill {
    pub fn git(
        name: impl Into<String>,
        source: impl Into<String>,
        commit: String,
        checksum: String,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Git { commit, checksum },
        }
    }

    pub fn binary(
        name: impl Into<String>,
        source: impl Into<String>,
        binary_name: impl Into<String>,
        binary_version: Option<String>,
        binary_checksum: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Binary {
                binary_name: binary_name.into(),
                binary_version,
                binary_checksum,
                dev: false,
            },
        }
    }

    pub fn local(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: String::new(),
            path: None,
            version: None,
            kind: LockedSkillKind::Local { checksum: None },
        }
    }

    pub fn http(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Http { checksum: None },
        }
    }

    pub fn path(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Path { checksum: None },
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        match &mut self.kind {
            LockedSkillKind::Local { checksum: c }
            | LockedSkillKind::Http { checksum: c }
            | LockedSkillKind::Path { checksum: c } => *c = Some(checksum.into()),
            _ => {}
        }
        self
    }

    pub fn with_dev(mut self) -> Self {
        match &mut self.kind {
            LockedSkillKind::Binary { dev, .. } => *dev = true,
            _ => panic!("with_dev() called on non-binary LockedSkill"),
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Convenience accessors
// ---------------------------------------------------------------------------

impl LockedSkill {
    pub fn is_binary(&self) -> bool {
        matches!(self.kind, LockedSkillKind::Binary { .. })
    }

    pub fn is_dev(&self) -> bool {
        matches!(self.kind, LockedSkillKind::Binary { dev: true, .. })
    }

    pub fn binary_name(&self) -> Option<&str> {
        match &self.kind {
            LockedSkillKind::Binary { binary_name, .. } => Some(binary_name),
            _ => None,
        }
    }

    pub fn binary_version(&self) -> Option<&str> {
        match &self.kind {
            LockedSkillKind::Binary { binary_version, .. } => binary_version.as_deref(),
            _ => None,
        }
    }

    pub fn commit(&self) -> Option<&str> {
        match &self.kind {
            LockedSkillKind::Git { commit, .. } => Some(commit),
            _ => None,
        }
    }

    pub fn checksum(&self) -> Option<&str> {
        match &self.kind {
            LockedSkillKind::Git { checksum, .. } => Some(checksum),
            LockedSkillKind::Local { checksum }
            | LockedSkillKind::Http { checksum }
            | LockedSkillKind::Path { checksum } => checksum.as_deref(),
            LockedSkillKind::Binary {
                binary_checksum, ..
            } => binary_checksum.as_deref(),
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy format (pre-0.3, no `kind` field)
// ---------------------------------------------------------------------------

/// Old lockfile entry format: flat struct, no `kind` discriminant,
/// `binary` instead of `binary_name`, `dev: Option<bool>`.
#[derive(Debug, Clone, Deserialize)]
struct LegacyLockedSkill {
    name: String,
    source: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    commit: Option<String>,
    #[serde(default)]
    checksum: Option<String>,
    /// Old field name for binary skills (renamed to `binary_name` in current format).
    #[serde(default)]
    binary: Option<String>,
    #[serde(default)]
    binary_version: Option<String>,
    #[serde(default)]
    binary_checksum: Option<String>,
    #[serde(default)]
    dev: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyLockfile {
    #[serde(default, rename = "skill")]
    skills: Vec<LegacyLockedSkill>,
    #[serde(default)]
    agents: Option<crate::agents::AgentsLockEntry>,
}

impl From<LegacyLockedSkill> for LockedSkill {
    fn from(old: LegacyLockedSkill) -> Self {
        let kind = if let Some(binary_name) = old.binary {
            LockedSkillKind::Binary {
                binary_name,
                binary_version: old.binary_version,
                binary_checksum: old.binary_checksum,
                dev: old.dev.unwrap_or(false),
            }
        } else {
            LockedSkillKind::Git {
                commit: old.commit.unwrap_or_default(),
                checksum: old.checksum.unwrap_or_default(),
            }
        };
        LockedSkill {
            name: old.name,
            source: old.source,
            path: old.path,
            version: old.version,
            kind,
        }
    }
}

// ---------------------------------------------------------------------------
// Serde bridge (private)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawLockedSkill {
    name: String,
    source: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_checksum: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    dev: bool,
}

fn is_false(v: &bool) -> bool {
    !v
}

impl TryFrom<RawLockedSkill> for LockedSkill {
    type Error = String;

    fn try_from(raw: RawLockedSkill) -> std::result::Result<Self, String> {
        let kind = match raw.kind.as_str() {
            "git" => LockedSkillKind::Git {
                commit: raw.commit.ok_or("git skill missing 'commit' field")?,
                checksum: raw.checksum.ok_or("git skill missing 'checksum' field")?,
            },
            "binary" => LockedSkillKind::Binary {
                binary_name: raw
                    .binary_name
                    .ok_or("binary skill missing 'binary_name' field")?,
                binary_version: raw.binary_version,
                binary_checksum: raw.binary_checksum,
                dev: raw.dev,
            },
            "local" => LockedSkillKind::Local {
                checksum: raw.checksum,
            },
            "http" => LockedSkillKind::Http {
                checksum: raw.checksum,
            },
            "path" => LockedSkillKind::Path {
                checksum: raw.checksum,
            },
            other => {
                return Err(format!(
                    "unknown locked skill kind '{other}' — you may need to update Ion"
                ));
            }
        };
        Ok(LockedSkill {
            name: raw.name,
            source: raw.source,
            path: raw.path,
            version: raw.version,
            kind,
        })
    }
}

impl From<LockedSkill> for RawLockedSkill {
    fn from(skill: LockedSkill) -> Self {
        let (kind_str, commit, checksum, binary_name, binary_version, binary_checksum, dev) =
            match skill.kind {
                LockedSkillKind::Git { commit, checksum } => {
                    ("git", Some(commit), Some(checksum), None, None, None, false)
                }
                LockedSkillKind::Binary {
                    binary_name,
                    binary_version,
                    binary_checksum,
                    dev,
                } => (
                    "binary",
                    None,
                    None,
                    Some(binary_name),
                    binary_version,
                    binary_checksum,
                    dev,
                ),
                LockedSkillKind::Local { checksum } => {
                    ("local", None, checksum, None, None, None, false)
                }
                LockedSkillKind::Http { checksum } => {
                    ("http", None, checksum, None, None, None, false)
                }
                LockedSkillKind::Path { checksum } => {
                    ("path", None, checksum, None, None, None, false)
                }
            };
        RawLockedSkill {
            name: skill.name,
            source: skill.source,
            kind: kind_str.to_string(),
            path: skill.path,
            version: skill.version,
            commit,
            checksum,
            binary_name,
            binary_version,
            binary_checksum,
            dev,
        }
    }
}

// ---------------------------------------------------------------------------
// Raw lockfile serde bridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RawLockfile {
    #[serde(default, rename = "skill")]
    skills: Vec<RawLockedSkill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agents: Option<crate::agents::AgentsLockEntry>,
}

// ---------------------------------------------------------------------------
// Lockfile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Lockfile {
    pub skills: Vec<LockedSkill>,
    pub agents: Option<crate::agents::AgentsLockEntry>,
}

impl Lockfile {
    pub fn from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;

        // Try current format first.
        match toml::from_str::<RawLockfile>(&content) {
            Ok(raw) => {
                let skills = raw
                    .skills
                    .into_iter()
                    .map(LockedSkill::try_from)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| Error::Manifest(format!("Invalid lockfile entry: {e}")))?;
                Ok(Lockfile {
                    skills,
                    agents: raw.agents,
                })
            }
            Err(current_err) => {
                // Try legacy format (no `kind` field, `binary` instead of `binary_name`).
                if let Ok(legacy) = toml::from_str::<LegacyLockfile>(&content) {
                    let lockfile = Lockfile {
                        skills: legacy.skills.into_iter().map(LockedSkill::from).collect(),
                        agents: legacy.agents,
                    };
                    // Rewrite in current format so future reads are fast.
                    if let Err(e) = lockfile.write_to(path) {
                        log::warn!("Failed to rewrite migrated lockfile: {e}");
                    } else {
                        eprintln!("Migrated Ion.lock to current format (added `kind` field).");
                    }
                    return Ok(lockfile);
                }

                Err(Error::Manifest(format!(
                    "Failed to parse lockfile: {current_err}. \
                     Try deleting Ion.lock and running `ion install` to regenerate it."
                )))
            }
        }
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        let raw = RawLockfile {
            skills: self
                .skills
                .iter()
                .cloned()
                .map(RawLockedSkill::from)
                .collect(),
            agents: self.agents.clone(),
        };
        let content = toml::to_string_pretty(&raw)
            .map_err(|e| Error::Manifest(format!("Failed to serialize lockfile: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)
    }

    pub fn find(&self, name: &str) -> Option<&LockedSkill> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn upsert(&mut self, skill: LockedSkill) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.name == skill.name) {
            *existing = skill;
        } else {
            self.skills.push(skill);
        }
        self.skills.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn remove(&mut self, name: &str) {
        self.skills.retain(|s| s.name != name);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lockfile() {
        let content = r#"
[[skill]]
name = "brainstorming"
source = "https://github.com/obra/superpowers.git"
kind = "git"
path = "brainstorming"
version = "1.0"
commit = "abc123"
checksum = "sha256:deadbeef"
"#;
        let raw: RawLockfile = toml::from_str(content).unwrap();
        let lockfile_skills: Vec<LockedSkill> = raw
            .skills
            .into_iter()
            .map(LockedSkill::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(lockfile_skills.len(), 1);
        assert_eq!(lockfile_skills[0].name, "brainstorming");
        assert_eq!(lockfile_skills[0].commit(), Some("abc123"));
    }

    #[test]
    fn roundtrip_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.upsert(
            LockedSkill::git(
                "my-skill",
                "https://github.com/org/repo.git",
                "abc123".into(),
                "sha256:deadbeef".into(),
            )
            .with_version("1.0"),
        );

        lockfile.write_to(&path).unwrap();
        let loaded = Lockfile::from_file(&path).unwrap();
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.skills[0], lockfile.skills[0]);
    }

    #[test]
    fn upsert_updates_existing() {
        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill::git("s", "a", "old".into(), "c1".into()));
        lockfile.upsert(LockedSkill::git("s", "a", "new".into(), "c2".into()));
        assert_eq!(lockfile.skills.len(), 1);
        assert_eq!(lockfile.skills[0].commit(), Some("new"));
    }

    #[test]
    fn remove_skill() {
        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill::local("a"));
        lockfile.remove("a");
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn roundtrip_binary_locked_skill() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.upsert(
            LockedSkill::binary(
                "mytool",
                "https://github.com/owner/mytool.git",
                "mytool",
                Some("1.2.0".into()),
                Some("sha256:abc123".into()),
            )
            .with_version("1.2.0"),
        );

        lockfile.write_to(&path).unwrap();
        let loaded = Lockfile::from_file(&path).unwrap();
        assert_eq!(loaded.skills[0].binary_name(), Some("mytool"));
        assert_eq!(loaded.skills[0].binary_version(), Some("1.2.0"));
        assert_eq!(loaded.skills[0].checksum(), Some("sha256:abc123"));
    }

    #[test]
    fn from_missing_file_returns_empty() {
        let lockfile = Lockfile::from_file(Path::new("/nonexistent/Ion.lock")).unwrap();
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn parse_lockfile_with_agents() {
        let content = r#"
[[skill]]
name = "brainstorming"
source = "https://github.com/obra/superpowers.git"
kind = "git"
path = "brainstorming"
commit = "abc123"
checksum = "sha256:beef"

[agents]
template = "org/agents-templates"
rev = "def456"
checksum = "sha256:deadbeef"
updated-at = "2026-03-27T00:00:00Z"
"#;
        let raw: RawLockfile = toml::from_str(content).unwrap();
        let skills: Vec<LockedSkill> = raw
            .skills
            .into_iter()
            .map(LockedSkill::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(skills.len(), 1);
        let agents = raw.agents.as_ref().unwrap();
        assert_eq!(agents.template, "org/agents-templates");
        assert_eq!(agents.rev.as_deref(), Some("def456"));
        assert_eq!(agents.checksum, "sha256:deadbeef");
        assert_eq!(agents.updated_at, "2026-03-27T00:00:00Z");
    }

    #[test]
    fn parse_lockfile_without_agents_is_backward_compatible() {
        let content = r#"
[[skill]]
name = "test"
source = "https://github.com/org/repo.git"
kind = "local"
"#;
        let raw: RawLockfile = toml::from_str(content).unwrap();
        assert!(raw.agents.is_none());
        let skills: Vec<LockedSkill> = raw
            .skills
            .into_iter()
            .map(LockedSkill::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn roundtrip_lockfile_with_agents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.agents = Some(crate::agents::AgentsLockEntry {
            template: "org/agents-templates".to_string(),
            rev: Some("abc123".to_string()),
            checksum: "sha256:deadbeef".to_string(),
            updated_at: "2026-03-27T00:00:00Z".to_string(),
        });

        lockfile.write_to(&path).unwrap();
        let loaded = Lockfile::from_file(&path).unwrap();
        let agents = loaded.agents.unwrap();
        assert_eq!(agents.template, "org/agents-templates");
        assert_eq!(agents.checksum, "sha256:deadbeef");
    }

    #[test]
    fn builder_with_path_and_version() {
        let skill = LockedSkill::git("s", "src", "c1".into(), "cs1".into())
            .with_path("sub/dir")
            .with_version("2.0");
        assert_eq!(skill.path.as_deref(), Some("sub/dir"));
        assert_eq!(skill.version.as_deref(), Some("2.0"));
    }

    #[test]
    fn builder_with_checksum_on_local() {
        let skill = LockedSkill::local("s").with_checksum("sha256:abc");
        assert_eq!(skill.checksum(), Some("sha256:abc"));
    }

    #[test]
    fn builder_with_dev_on_binary() {
        let skill = LockedSkill::binary("s", "src", "bin", None, None).with_dev();
        match &skill.kind {
            LockedSkillKind::Binary { dev, .. } => assert!(dev),
            _ => panic!("expected binary kind"),
        }
    }

    #[test]
    fn accessors_return_none_for_wrong_kind() {
        let skill = LockedSkill::local("s");
        assert!(!skill.is_binary());
        assert!(skill.binary_name().is_none());
        assert!(skill.binary_version().is_none());
        assert!(skill.commit().is_none());
    }

    #[test]
    fn unknown_kind_gives_error() {
        let raw = RawLockedSkill {
            name: "x".into(),
            source: "y".into(),
            kind: "unknown".into(),
            path: None,
            version: None,
            commit: None,
            checksum: None,
            binary_name: None,
            binary_version: None,
            binary_checksum: None,
            dev: false,
        };
        let err = LockedSkill::try_from(raw).unwrap_err();
        assert!(err.contains("unknown locked skill kind 'unknown'"));
    }

    #[test]
    fn git_missing_commit_gives_error() {
        let raw = RawLockedSkill {
            name: "x".into(),
            source: "y".into(),
            kind: "git".into(),
            path: None,
            version: None,
            commit: None,
            checksum: Some("sha256:abc".into()),
            binary_name: None,
            binary_version: None,
            binary_checksum: None,
            dev: false,
        };
        let err = LockedSkill::try_from(raw).unwrap_err();
        assert!(err.contains("missing 'commit'"));
    }

    #[test]
    fn migrate_legacy_git_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        // Old format: no `kind` field
        std::fs::write(
            &path,
            r#"
[[skill]]
name = "brainstorming"
source = "https://github.com/obra/superpowers.git"
path = "brainstorming"
version = "1.0"
commit = "abc123"
checksum = "sha256:deadbeef"
"#,
        )
        .unwrap();

        let lockfile = Lockfile::from_file(&path).unwrap();
        assert_eq!(lockfile.skills.len(), 1);
        assert_eq!(lockfile.skills[0].name, "brainstorming");
        assert_eq!(lockfile.skills[0].commit(), Some("abc123"));
        assert_eq!(lockfile.skills[0].checksum(), Some("sha256:deadbeef"));
        assert_eq!(lockfile.skills[0].path.as_deref(), Some("brainstorming"));
        assert!(matches!(
            lockfile.skills[0].kind,
            LockedSkillKind::Git { .. }
        ));

        // File should have been rewritten with `kind` field
        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert!(rewritten.contains("kind = \"git\""));
    }

    #[test]
    fn migrate_legacy_binary_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        // Old format: `binary` field instead of `binary_name`, `dev` as bool
        std::fs::write(
            &path,
            r#"
[[skill]]
name = "mytool"
source = "https://github.com/owner/mytool.git"
binary = "mytool-bin"
binary_version = "1.2.0"
binary_checksum = "sha256:abc123"
dev = true
"#,
        )
        .unwrap();

        let lockfile = Lockfile::from_file(&path).unwrap();
        assert_eq!(lockfile.skills.len(), 1);
        assert_eq!(lockfile.skills[0].binary_name(), Some("mytool-bin"));
        assert_eq!(lockfile.skills[0].binary_version(), Some("1.2.0"));
        assert!(lockfile.skills[0].is_dev());
        assert!(matches!(
            lockfile.skills[0].kind,
            LockedSkillKind::Binary { .. }
        ));

        // File should have been rewritten
        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert!(rewritten.contains("kind = \"binary\""));
        assert!(rewritten.contains("binary_name = \"mytool-bin\""));
    }

    #[test]
    fn migrate_legacy_mixed_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        std::fs::write(
            &path,
            r#"
[[skill]]
name = "a-git-skill"
source = "https://github.com/org/repo.git"
commit = "aaa"
checksum = "sha256:bbb"

[[skill]]
name = "b-binary-skill"
source = "https://github.com/owner/tool.git"
binary = "tool"
binary_version = "2.0"
"#,
        )
        .unwrap();

        let lockfile = Lockfile::from_file(&path).unwrap();
        assert_eq!(lockfile.skills.len(), 2);
        assert!(matches!(
            lockfile.skills[0].kind,
            LockedSkillKind::Git { .. }
        ));
        assert!(matches!(
            lockfile.skills[1].kind,
            LockedSkillKind::Binary { .. }
        ));
    }

    #[test]
    fn migrate_legacy_preserves_agents_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        std::fs::write(
            &path,
            r#"
[[skill]]
name = "my-skill"
source = "https://github.com/org/repo.git"
commit = "abc"
checksum = "sha256:def"

[agents]
template = "org/templates"
checksum = "sha256:aaa"
updated-at = "2026-01-01T00:00:00Z"
"#,
        )
        .unwrap();

        let lockfile = Lockfile::from_file(&path).unwrap();
        assert_eq!(lockfile.skills.len(), 1);
        let agents = lockfile.agents.as_ref().unwrap();
        assert_eq!(agents.template, "org/templates");
    }
}
