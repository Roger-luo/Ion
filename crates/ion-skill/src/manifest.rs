use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::{SkillSource, SourceType};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillEntry {
    Shorthand(String),
    Full {
        #[serde(rename = "type", default)]
        source_type: Option<SourceType>,
        #[serde(default)]
        source: Option<String>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        binary: Option<String>,
        #[serde(default, alias = "asset-pattern")]
        asset_pattern: Option<String>,
        #[serde(default, alias = "forked-from")]
        forked_from: Option<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestOptions {
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    #[serde(default)]
    pub skills_dir: Option<String>,
}

impl ManifestOptions {
    /// Get a project config value by key. Supports dot-notation for targets
    /// and top-level keys like "skills-dir".
    pub fn get_value(&self, key: &str) -> Option<String> {
        if key == "skills-dir" {
            return self.skills_dir.clone();
        }
        let (section, field) = key.split_once('.')?;
        match section {
            "targets" => self.targets.get(field).cloned(),
            _ => None,
        }
    }

    /// List all project config values as (key, value) pairs.
    pub fn list_values(&self) -> Vec<(String, String)> {
        let mut values: Vec<(String, String)> = self
            .targets
            .iter()
            .map(|(k, v)| (format!("targets.{k}"), v.clone()))
            .collect();
        if let Some(dir) = &self.skills_dir {
            values.push(("skills-dir".to_string(), dir.clone()));
        }
        values
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub skills: BTreeMap<String, SkillEntry>,
    #[serde(default)]
    pub options: ManifestOptions,
}

impl Manifest {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        // Check for deprecated options before parsing
        let raw: toml::Value = toml::from_str(content).map_err(Error::TomlParse)?;
        if let Some(options) = raw.get("options")
            && options.get("install-to-claude").is_some()
        {
            return Err(Error::Manifest(
                "'install-to-claude' is no longer supported. Use [options.targets] instead:\n\n\
                 [options.targets]\n\
                 claude = \".claude/skills\"\n"
                    .to_string(),
            ));
        }

        toml::from_str(content).map_err(Error::TomlParse)
    }

    pub fn resolve_entry(entry: &SkillEntry) -> Result<SkillSource> {
        match entry {
            SkillEntry::Shorthand(s) => SkillSource::infer(s),
            SkillEntry::Full {
                source_type,
                source,
                version,
                rev,
                path,
                binary,
                asset_pattern,
                forked_from,
            } => {
                let mut resolved = match source_type {
                    Some(SourceType::Local) => SkillSource {
                        source_type: SourceType::Local,
                        source: source.clone().unwrap_or_default(),
                        path: path.clone(),
                        rev: None,
                        version: None,
                        binary: None,
                        asset_pattern: None,
                        forked_from: None,
                    },
                    Some(st) => {
                        let src = source.as_deref().ok_or_else(|| {
                            Error::Manifest(format!(
                                "source is required for type {:?}",
                                st
                            ))
                        })?;
                        SkillSource {
                            source_type: st.clone(),
                            source: src.to_string(),
                            path: path.clone(),
                            rev: None,
                            version: None,
                            binary: None,
                            asset_pattern: None,
                            forked_from: None,
                        }
                    }
                    None => {
                        let src = source.as_deref().ok_or_else(|| {
                            Error::Manifest(
                                "source is required when type is not specified".to_string(),
                            )
                        })?;
                        SkillSource::infer(src)?
                    }
                };
                if let Some(v) = version {
                    resolved.version = Some(v.clone());
                }
                if let Some(r) = rev {
                    resolved.rev = Some(r.clone());
                }
                if path.is_some() {
                    resolved.path = path.clone();
                }
                resolved.binary = binary.clone();
                resolved.asset_pattern = asset_pattern.clone();
                resolved.forked_from = forked_from.clone();
                Ok(resolved)
            }
        }
    }

    pub fn empty() -> Self {
        Self {
            skills: BTreeMap::new(),
            options: ManifestOptions::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shorthand_entry() {
        let toml_str = "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["brainstorming"]).unwrap();
        assert_eq!(source.source_type, SourceType::Github);
        assert_eq!(source.source, "anthropics/skills");
        assert_eq!(source.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn parse_full_github_entry() {
        let toml_str =
            "[skills]\nmy-tool = { type = \"github\", source = \"org/skills/my-tool\", rev = \"v2.0\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-tool"]).unwrap();
        assert_eq!(source.source_type, SourceType::Github);
        assert_eq!(source.rev.as_deref(), Some("v2.0"));
    }

    #[test]
    fn parse_full_git_entry() {
        let toml_str = "[skills]\ngitlab-skill = { type = \"git\", source = \"https://gitlab.com/org/skills.git\", path = \"my-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["gitlab-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Git);
        assert_eq!(source.path.as_deref(), Some("my-skill"));
    }

    #[test]
    fn parse_local_path_entry() {
        let toml_str =
            "[skills]\nlocal-skill = { type = \"path\", source = \"../my-local-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["local-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Path);
    }

    #[test]
    fn parse_options() {
        let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert_eq!(manifest.options.targets.len(), 1);
        assert_eq!(manifest.options.targets["claude"], ".claude/skills");
    }

    #[test]
    fn parse_targets_options() {
        let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert_eq!(manifest.options.targets.len(), 2);
        assert_eq!(manifest.options.targets["claude"], ".claude/skills");
        assert_eq!(manifest.options.targets["cursor"], ".cursor/skills");
    }

    #[test]
    fn parse_empty_manifest() {
        let manifest = Manifest::parse("[skills]\n").unwrap();
        assert!(manifest.skills.is_empty());
        assert!(manifest.options.targets.is_empty());
    }

    #[test]
    fn rejects_old_install_to_claude_option() {
        let toml_str = "[skills]\n\n[options]\ninstall-to-claude = true\n";
        let result = Manifest::parse(toml_str);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("install-to-claude"),
            "Error should mention the old option: {err_msg}"
        );
    }

    #[test]
    fn parse_version_entry() {
        let toml_str = "[skills]\nmy-skill = { type = \"github\", source = \"org/repo/my-skill\", version = \"1.0\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-skill"]).unwrap();
        assert_eq!(source.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn get_project_value() {
        let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert_eq!(
            manifest.options.get_value("targets.claude"),
            Some(".claude/skills".to_string())
        );
        assert_eq!(manifest.options.get_value("targets.nonexistent"), None);
    }

    #[test]
    fn parse_binary_skill_entry() {
        let toml_str = "[skills]\nmytool = { type = \"binary\", source = \"owner/mytool\", binary = \"mytool\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["mytool"]).unwrap();
        assert_eq!(source.source_type, SourceType::Binary);
        assert_eq!(source.source, "owner/mytool");
        assert_eq!(source.binary.as_deref(), Some("mytool"));
    }

    #[test]
    fn parse_asset_pattern_manifest() {
        let toml_str = r#"[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", asset-pattern = "mytool-{version}-{os}-{arch}.tar.gz" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["mytool"]).unwrap();
        assert_eq!(
            source.asset_pattern.as_deref(),
            Some("mytool-{version}-{os}-{arch}.tar.gz")
        );
    }

    #[test]
    fn list_project_values() {
        let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let values = manifest.options.list_values();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&("targets.claude".to_string(), ".claude/skills".to_string())));
    }

    #[test]
    fn parse_local_skill_entry() {
        let toml_str = "[skills]\nmy-skill = { type = \"local\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Local);
        assert_eq!(source.source, "");
        assert!(source.forked_from.is_none());
    }

    #[test]
    fn parse_local_skill_with_forked_from() {
        let toml_str =
            "[skills]\nmy-skill = { type = \"local\", forked-from = \"org/original-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = Manifest::resolve_entry(&manifest.skills["my-skill"]).unwrap();
        assert_eq!(source.source_type, SourceType::Local);
        assert_eq!(
            source.forked_from.as_deref(),
            Some("org/original-skill")
        );
    }

    #[test]
    fn parse_skills_dir_option() {
        let toml_str = "[skills]\n\n[options]\nskills-dir = \"my-skills\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert_eq!(manifest.options.skills_dir.as_deref(), Some("my-skills"));
        assert_eq!(
            manifest.options.get_value("skills-dir"),
            Some("my-skills".to_string())
        );
    }
}
