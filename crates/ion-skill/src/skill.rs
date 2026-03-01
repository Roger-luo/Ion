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
            serde_yaml::from_str(yaml).map_err(Error::YamlParse)?;

        Self::validate_name(&meta.name)?;

        if meta.description.is_empty() {
            return Err(Error::InvalidSkill(
                "description must not be empty".to_string(),
            ));
        }

        Ok((meta, body))
    }

    /// Parse SKILL.md from a file path.
    pub fn from_file(path: &Path) -> Result<(Self, String)> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
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
        assert_eq!(meta.metadata.as_ref().unwrap().get("author").unwrap(), "test-org");
    }

    #[test]
    fn parse_skill_missing_frontmatter() {
        let content = "# No frontmatter here\n\nJust markdown.\n";
        assert!(SkillMetadata::parse(content).is_err());
    }

    #[test]
    fn parse_skill_missing_name() {
        let content = "---\ndescription: No name field.\n---\n\nBody.\n";
        assert!(SkillMetadata::parse(content).is_err());
    }

    #[test]
    fn validate_good_names() {
        assert!(SkillMetadata::validate_name("pdf-processing").is_ok());
        assert!(SkillMetadata::validate_name("a").is_ok());
        assert!(SkillMetadata::validate_name("data-analysis").is_ok());
    }

    #[test]
    fn validate_bad_names() {
        assert!(SkillMetadata::validate_name("PDF-Processing").is_err());
        assert!(SkillMetadata::validate_name("-pdf").is_err());
        assert!(SkillMetadata::validate_name("pdf-").is_err());
        assert!(SkillMetadata::validate_name("pdf--processing").is_err());
        assert!(SkillMetadata::validate_name("").is_err());
        assert!(SkillMetadata::validate_name(&"a".repeat(65)).is_err());
    }
}
