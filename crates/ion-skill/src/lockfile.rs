use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_checksum: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(default, rename = "skill")]
    pub skills: Vec<LockedSkill>,
}

impl Lockfile {
    pub fn from_file(path: &Path) -> Result<Self> {
        crate::load_toml_or_default(path)
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
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
        let path = dir.path().join("Ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill {
            name: "my-skill".to_string(),
            source: "https://github.com/org/repo.git".to_string(),
            path: None,
            version: Some("1.0".to_string()),
            commit: Some("abc123".to_string()),
            checksum: Some("sha256:deadbeef".to_string()),
            binary: None,
            binary_version: None,
            binary_checksum: None,
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
            binary: None,
            binary_version: None,
            binary_checksum: None,
        });
        lockfile.upsert(LockedSkill {
            name: "s".to_string(),
            source: "a".to_string(),
            path: None,
            version: None,
            commit: Some("new".to_string()),
            checksum: None,
            binary: None,
            binary_version: None,
            binary_checksum: None,
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
            binary: None,
            binary_version: None,
            binary_checksum: None,
        });
        lockfile.remove("a");
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn roundtrip_binary_locked_skill() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.lock");

        let mut lockfile = Lockfile::default();
        lockfile.upsert(LockedSkill {
            name: "mytool".to_string(),
            source: "https://github.com/owner/mytool.git".to_string(),
            path: None,
            version: Some("1.2.0".to_string()),
            commit: None,
            checksum: None,
            binary: Some("mytool".to_string()),
            binary_version: Some("1.2.0".to_string()),
            binary_checksum: Some("sha256:abc123".to_string()),
        });

        lockfile.write_to(&path).unwrap();
        let loaded = Lockfile::from_file(&path).unwrap();
        assert_eq!(loaded.skills[0].binary.as_deref(), Some("mytool"));
        assert_eq!(loaded.skills[0].binary_version.as_deref(), Some("1.2.0"));
        assert_eq!(
            loaded.skills[0].binary_checksum.as_deref(),
            Some("sha256:abc123")
        );
    }

    #[test]
    fn from_missing_file_returns_empty() {
        let lockfile = Lockfile::from_file(Path::new("/nonexistent/Ion.lock")).unwrap();
        assert!(lockfile.skills.is_empty());
    }
}
