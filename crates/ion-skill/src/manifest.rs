use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::{SkillSource, SourceType};
use crate::{Error, Result};

/// Default directory where skills are installed within a project.
pub const DEFAULT_SKILLS_DIR: &str = ".agents/skills";

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
        #[serde(default)]
        dev: Option<bool>,
    },
}

impl SkillEntry {
    /// Resolve this manifest entry into a fully qualified SkillSource.
    pub fn resolve(&self) -> Result<SkillSource> {
        match self {
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
                dev,
            } => {
                let mut resolved = match source_type {
                    Some(SourceType::Local) => {
                        let mut s =
                            SkillSource::new(SourceType::Local, source.clone().unwrap_or_default());
                        s.path = path.clone();
                        s
                    }
                    Some(st) => {
                        let src = source.as_deref().ok_or_else(|| {
                            Error::Manifest(format!("source is required for type {:?}", st))
                        })?;
                        let mut s = SkillSource::new(st.clone(), src);
                        s.path = path.clone();
                        s
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
                // Apply kind-specific fields via builder methods
                if let Some(b) = binary {
                    resolved = resolved.with_binary(b.as_str());
                }
                if let Some(ap) = asset_pattern {
                    resolved = resolved.with_asset_pattern(ap.as_str());
                }
                if let Some(ff) = forked_from {
                    resolved = resolved.with_forked_from(ff.as_str());
                }
                if dev.unwrap_or(false) {
                    resolved = resolved.with_dev(true);
                }
                Ok(resolved)
            }
        }
    }
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

    /// Returns the configured skills directory, or the default `.agents/skills`.
    pub fn skills_dir_or_default(&self) -> &str {
        self.skills_dir.as_deref().unwrap_or(DEFAULT_SKILLS_DIR)
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

/// Workspace configuration for multi-project setups.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub members: Vec<String>,
}

/// Metadata about the project itself (not its dependencies).
///
/// Present in `[project]` section of Ion.toml for projects that are themselves
/// skills (e.g. binary skill projects). Optional — most Ion.toml files only
/// have `[skills]` and `[options]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// The project type: "binary" for binary skill projects.
    #[serde(rename = "type", default)]
    pub project_type: Option<String>,
    /// Override the binary executable name (defaults to Cargo.toml package name).
    #[serde(default)]
    pub binary: Option<String>,
}

impl ProjectMeta {
    /// Returns true if this project declares itself as a binary skill.
    pub fn is_binary(&self) -> bool {
        self.project_type.as_deref() == Some("binary")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub project: Option<ProjectMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    pub skills: BTreeMap<String, SkillEntry>,
    #[serde(default)]
    pub options: ManifestOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<crate::agents::AgentsConfig>,
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

    /// Resolve a manifest entry into a SkillSource.
    ///
    /// Prefer calling `entry.resolve()` directly. This static method is kept
    /// for backward compatibility.
    pub fn resolve_entry(entry: &SkillEntry) -> Result<SkillSource> {
        entry.resolve()
    }

    pub fn empty() -> Self {
        Self {
            project: None,
            workspace: None,
            skills: BTreeMap::new(),
            options: ManifestOptions::default(),
            agents: None,
        }
    }
}

/// Read just the `[project]` section from an Ion.toml file, if present.
///
/// Returns `None` if the file doesn't exist, can't be parsed, or has no
/// `[project]` section. This is intentionally lenient — it's used for
/// auto-detection, not validation.
pub fn read_project_meta(path: &Path) -> Option<ProjectMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let manifest: Manifest = toml::from_str(&content).ok()?;
    manifest.project
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SkillSourceKind;

    #[test]
    fn parse_shorthand_entry() {
        let toml_str = "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["brainstorming"].resolve().unwrap();
        assert_eq!(source.kind, SkillSourceKind::Github);
        assert_eq!(source.source, "obra/superpowers");
        assert_eq!(source.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn parse_full_github_entry() {
        let toml_str = "[skills]\nmy-tool = { type = \"github\", source = \"org/skills/my-tool\", rev = \"v2.0\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["my-tool"].resolve().unwrap();
        assert_eq!(source.kind, SkillSourceKind::Github);
        assert_eq!(source.rev.as_deref(), Some("v2.0"));
    }

    #[test]
    fn parse_full_git_entry() {
        let toml_str = "[skills]\ngitlab-skill = { type = \"git\", source = \"https://gitlab.com/org/skills.git\", path = \"my-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["gitlab-skill"].resolve().unwrap();
        assert_eq!(source.kind, SkillSourceKind::Git);
        assert_eq!(source.path.as_deref(), Some("my-skill"));
    }

    #[test]
    fn parse_local_path_entry() {
        let toml_str =
            "[skills]\nlocal-skill = { type = \"path\", source = \"../my-local-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["local-skill"].resolve().unwrap();
        assert_eq!(source.kind, SkillSourceKind::Path);
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
        let source = manifest.skills["my-skill"].resolve().unwrap();
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
        let source = manifest.skills["mytool"].resolve().unwrap();
        assert!(source.is_binary());
        assert_eq!(source.source, "owner/mytool");
        assert!(
            matches!(&source.kind, SkillSourceKind::Binary { binary_name, .. } if binary_name == "mytool")
        );
    }

    #[test]
    fn parse_asset_pattern_manifest() {
        let toml_str = r#"[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", asset-pattern = "mytool-{version}-{os}-{arch}.tar.gz" }
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["mytool"].resolve().unwrap();
        assert!(matches!(
            &source.kind,
            SkillSourceKind::Binary { asset_pattern: Some(ap), .. } if ap == "mytool-{version}-{os}-{arch}.tar.gz"
        ));
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
        let source = manifest.skills["my-skill"].resolve().unwrap();
        assert!(source.is_local());
        assert_eq!(source.source, "");
        assert!(matches!(
            &source.kind,
            SkillSourceKind::Local { forked_from: None }
        ));
    }

    #[test]
    fn parse_local_skill_with_forked_from() {
        let toml_str =
            "[skills]\nmy-skill = { type = \"local\", forked-from = \"org/original-skill\" }\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let source = manifest.skills["my-skill"].resolve().unwrap();
        assert!(source.is_local());
        assert!(
            matches!(&source.kind, SkillSourceKind::Local { forked_from: Some(ff) } if ff == "org/original-skill")
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

    #[test]
    fn parse_agents_config() {
        let toml_str = r#"
[skills]

[agents]
template = "org/agents-templates"
rev = "v2.0"
path = "templates/AGENTS.md"
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let agents = manifest.agents.as_ref().unwrap();
        assert_eq!(agents.template.as_deref(), Some("org/agents-templates"));
        assert_eq!(agents.rev.as_deref(), Some("v2.0"));
        assert_eq!(agents.path.as_deref(), Some("templates/AGENTS.md"));
    }

    #[test]
    fn parse_manifest_without_agents() {
        let toml_str = "[skills]\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert!(manifest.agents.is_none());
    }

    #[test]
    fn skills_dir_or_default_uses_default() {
        let opts = ManifestOptions {
            targets: std::collections::BTreeMap::new(),
            skills_dir: None,
        };
        assert_eq!(opts.skills_dir_or_default(), ".agents/skills");
    }

    #[test]
    fn skills_dir_or_default_uses_custom() {
        let opts = ManifestOptions {
            targets: std::collections::BTreeMap::new(),
            skills_dir: Some("custom/skills".to_string()),
        };
        assert_eq!(opts.skills_dir_or_default(), "custom/skills");
    }

    #[test]
    fn parse_workspace_config() {
        let toml_str = r#"
[workspace]
members = ["docs", "packages/frontend"]

[skills]
foo = "bar/baz"
"#;
        let manifest = Manifest::parse(toml_str).unwrap();
        let ws = manifest
            .workspace
            .as_ref()
            .expect("workspace should be present");
        assert_eq!(ws.members, vec!["docs", "packages/frontend"]);
    }

    #[test]
    fn parse_manifest_without_workspace() {
        let toml_str = "[skills]\nfoo = \"bar/baz\"\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        assert!(manifest.workspace.is_none());
    }

    #[test]
    fn parse_empty_workspace_members() {
        let toml_str = "[workspace]\nmembers = []\n\n[skills]\n";
        let manifest = Manifest::parse(toml_str).unwrap();
        let ws = manifest.workspace.as_ref().unwrap();
        assert!(ws.members.is_empty());
    }
}
