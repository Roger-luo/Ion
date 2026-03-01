use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalConfig {
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryConfig>,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CacheConfig {
    pub max_age_days: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiConfig {
    pub color: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RegistryConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SearchConfig {
    pub agent_command: Option<String>,
}

impl GlobalConfig {
    /// Returns the platform-appropriate path for the global config file.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ion").join("config.toml"))
    }

    /// Load global config from the platform default path.
    /// Returns Default if the file doesn't exist.
    pub fn load() -> Result<Self> {
        match Self::config_path() {
            Some(path) => Self::load_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load global config from a specific path.
    /// Returns Default if the file doesn't exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).map_err(Error::TomlParse),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Merge global targets with project targets. Project wins on key collision.
    pub fn resolve_targets(&self, project: &crate::manifest::ManifestOptions) -> BTreeMap<String, String> {
        let mut merged = self.targets.clone();
        for (key, value) in &project.targets {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }

    /// Expand source aliases. If the first segment of a shorthand matches a source
    /// alias, replace it with the alias value. URLs and paths pass through unchanged.
    pub fn resolve_source(&self, input: &str) -> String {
        // Don't expand URLs or local paths
        if input.starts_with("https://")
            || input.starts_with("http://")
            || input.starts_with('/')
            || input.starts_with("./")
            || input.starts_with("../")
        {
            return input.to_string();
        }

        // Check if the first segment is an alias
        let segments: Vec<&str> = input.splitn(2, '/').collect();
        if segments.len() == 2
            && let Some(expanded) = self.sources.get(segments[0])
        {
            return format!("{}/{}", expanded, segments[1]);
        }

        input.to_string()
    }

    /// Save global config to a specific path. Creates parent directories.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Manifest(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }

    /// Get a config value by dot-notation key (e.g., "targets.claude", "ui.color").
    pub fn get_value(&self, key: &str) -> Option<String> {
        let (section, field) = key.split_once('.')?;
        match section {
            "targets" => self.targets.get(field).cloned(),
            "sources" => self.sources.get(field).cloned(),
            "cache" => match field {
                "max-age-days" => self.cache.max_age_days.map(|v| v.to_string()),
                _ => None,
            },
            "ui" => match field {
                "color" => self.ui.color.map(|v| v.to_string()),
                _ => None,
            },
            "registries" => self.registries.get(field).map(|r| r.url.clone()),
            "search" => match field {
                "agent-command" => self.search.agent_command.clone(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Set a config value in a TOML file by dot-notation key, preserving formatting.
    pub fn set_value_in_file(path: &Path, key: &str, value: &str) -> Result<()> {
        use toml_edit::{DocumentMut, Item, Table};

        let (section, field) = key.split_once('.').ok_or_else(|| {
            Error::Manifest(format!("Invalid key format '{key}': expected 'section.key'"))
        })?;

        match section {
            "targets" | "sources" | "cache" | "ui" | "registries" | "search" => {}
            _ => {
                return Err(Error::Manifest(format!(
                    "Unknown config section '{section}'. Valid sections: targets, sources, cache, ui, registries, search"
                )));
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

        if !doc.contains_key(section) {
            doc[section] = Item::Table(Table::new());
        }

        match (section, field) {
            ("cache", "max-age-days") => {
                let num: i64 = value.parse().map_err(|_| {
                    Error::Manifest(format!("'{value}' is not a valid integer for {key}"))
                })?;
                doc[section][field] = toml_edit::value(num);
            }
            ("ui", "color") => {
                let b: bool = value.parse().map_err(|_| {
                    Error::Manifest(format!("'{value}' is not a valid boolean for {key}"))
                })?;
                doc[section][field] = toml_edit::value(b);
            }
            ("registries", _) => {
                use toml_edit::InlineTable;
                let mut t = InlineTable::new();
                t.insert("url", value.into());
                doc[section][field] = toml_edit::Item::Value(toml_edit::Value::InlineTable(t));
            }
            _ => {
                doc[section][field] = toml_edit::value(value);
            }
        }

        std::fs::write(path, doc.to_string()).map_err(Error::Io)?;
        Ok(())
    }

    /// Delete a config value from a TOML file by dot-notation key, preserving formatting.
    pub fn delete_value_in_file(path: &Path, key: &str) -> Result<()> {
        use toml_edit::DocumentMut;

        let (section, field) = key.split_once('.').ok_or_else(|| {
            Error::Manifest(format!("Invalid key format '{key}': expected 'section.key'"))
        })?;

        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

        if let Some(table) = doc.get_mut(section).and_then(|item| item.as_table_mut()) {
            table.remove(field);
        }

        std::fs::write(path, doc.to_string()).map_err(Error::Io)?;
        Ok(())
    }

    /// List all config values as a Vec of (dot-key, value) pairs.
    pub fn list_values(&self) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        for (k, v) in &self.targets {
            entries.push((format!("targets.{k}"), v.clone()));
        }
        for (k, v) in &self.sources {
            entries.push((format!("sources.{k}"), v.clone()));
        }
        if let Some(days) = self.cache.max_age_days {
            entries.push(("cache.max-age-days".to_string(), days.to_string()));
        }
        if let Some(color) = self.ui.color {
            entries.push(("ui.color".to_string(), color.to_string()));
        }
        for (k, v) in &self.registries {
            entries.push((format!("registries.{k}"), v.url.clone()));
        }
        if let Some(ref cmd) = self.search.agent_command {
            entries.push(("search.agent-command".to_string(), cmd.clone()));
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = GlobalConfig::load_from(&path).unwrap();
        assert!(config.targets.is_empty());
        assert!(config.sources.is_empty());
        assert_eq!(config.cache.max_age_days, None);
        assert_eq!(config.ui.color, None);
    }

    #[test]
    fn load_parses_all_sections() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, r#"
[targets]
claude = ".claude/skills"
cursor = ".cursor/skills"

[sources]
superpowers = "obra/superpowers"

[cache]
max-age-days = 30

[ui]
color = true
"#).unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.targets.len(), 2);
        assert_eq!(config.targets["claude"], ".claude/skills");
        assert_eq!(config.sources["superpowers"], "obra/superpowers");
        assert_eq!(config.cache.max_age_days, Some(30));
        assert_eq!(config.ui.color, Some(true));
    }

    #[test]
    fn load_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\n").unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.targets.len(), 1);
        assert!(config.sources.is_empty());
        assert_eq!(config.cache.max_age_days, None);
    }

    #[test]
    fn resolve_targets_merges_global_and_project() {
        let mut global = GlobalConfig::default();
        global.targets.insert("claude".to_string(), ".claude/skills".to_string());
        global.targets.insert("cursor".to_string(), ".cursor/skills".to_string());

        let mut project = crate::manifest::ManifestOptions::default();
        project.targets.insert("claude".to_string(), ".claude/custom".to_string());

        let merged = global.resolve_targets(&project);
        // Project wins on collision
        assert_eq!(merged["claude"], ".claude/custom");
        // Global fills gaps
        assert_eq!(merged["cursor"], ".cursor/skills");
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn resolve_targets_empty_global() {
        let global = GlobalConfig::default();
        let mut project = crate::manifest::ManifestOptions::default();
        project.targets.insert("claude".to_string(), ".claude/skills".to_string());

        let merged = global.resolve_targets(&project);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged["claude"], ".claude/skills");
    }

    #[test]
    fn resolve_source_expands_alias() {
        let mut global = GlobalConfig::default();
        global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

        assert_eq!(
            global.resolve_source("superpowers/brainstorming"),
            "obra/superpowers/brainstorming"
        );
    }

    #[test]
    fn resolve_source_passes_through_unknown() {
        let global = GlobalConfig::default();

        assert_eq!(
            global.resolve_source("obra/superpowers/brainstorming"),
            "obra/superpowers/brainstorming"
        );
    }

    #[test]
    fn resolve_source_passes_through_urls() {
        let mut global = GlobalConfig::default();
        global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

        assert_eq!(
            global.resolve_source("https://github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
    }

    #[test]
    fn resolve_source_passes_through_paths() {
        let mut global = GlobalConfig::default();
        global.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());

        assert_eq!(
            global.resolve_source("./local-skill"),
            "./local-skill"
        );
    }

    #[test]
    fn save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = GlobalConfig::default();
        config.targets.insert("claude".to_string(), ".claude/skills".to_string());
        config.sources.insert("superpowers".to_string(), "obra/superpowers".to_string());
        config.cache.max_age_days = Some(7);
        config.ui.color = Some(false);

        config.save_to(&path).unwrap();

        let reloaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(reloaded.targets["claude"], ".claude/skills");
        assert_eq!(reloaded.sources["superpowers"], "obra/superpowers");
        assert_eq!(reloaded.cache.max_age_days, Some(7));
        assert_eq!(reloaded.ui.color, Some(false));
    }

    #[test]
    fn get_value_dot_notation() {
        let mut config = GlobalConfig::default();
        config.targets.insert("claude".to_string(), ".claude/skills".to_string());
        config.cache.max_age_days = Some(30);
        config.ui.color = Some(true);

        assert_eq!(config.get_value("targets.claude"), Some(".claude/skills".to_string()));
        assert_eq!(config.get_value("cache.max-age-days"), Some("30".to_string()));
        assert_eq!(config.get_value("ui.color"), Some("true".to_string()));
        assert_eq!(config.get_value("targets.nonexistent"), None);
        assert_eq!(config.get_value("invalid"), None);
    }

    #[test]
    fn set_value_preserves_formatting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "# My config\n[targets]\nclaude = \".claude/skills\"\n").unwrap();

        GlobalConfig::set_value_in_file(&path, "targets.cursor", ".cursor/skills").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# My config"));
        assert!(content.contains("cursor"));
        assert!(content.contains(".cursor/skills"));
        assert!(content.contains("claude"));
    }

    #[test]
    fn set_value_creates_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();

        GlobalConfig::set_value_in_file(&path, "targets.claude", ".claude/skills").unwrap();

        let reloaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(reloaded.targets["claude"], ".claude/skills");
    }

    #[test]
    fn set_value_cache_and_ui() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();

        GlobalConfig::set_value_in_file(&path, "cache.max-age-days", "7").unwrap();
        GlobalConfig::set_value_in_file(&path, "ui.color", "false").unwrap();

        let reloaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(reloaded.cache.max_age_days, Some(7));
        assert_eq!(reloaded.ui.color, Some(false));
    }

    #[test]
    fn delete_value_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n").unwrap();

        GlobalConfig::delete_value_in_file(&path, "targets.cursor").unwrap();

        let reloaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(reloaded.targets.len(), 1);
        assert!(reloaded.targets.contains_key("claude"));
    }

    #[test]
    fn set_value_invalid_key_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();

        let result = GlobalConfig::set_value_in_file(&path, "invalid", "value");
        assert!(result.is_err());
    }

    #[test]
    fn load_registries_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, r#"
[registries.skills-sh]
url = "https://skills.sh/api"
default = true

[registries.my-company]
url = "https://skills.internal.co/api"

[search]
agent-command = "claude -p 'search: {query}'"
"#).unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.registries.len(), 2);
        assert_eq!(config.registries["skills-sh"].url, "https://skills.sh/api");
        assert_eq!(config.registries["skills-sh"].default, Some(true));
        assert_eq!(config.search.agent_command, Some("claude -p 'search: {query}'".to_string()));
    }

    #[test]
    fn load_config_without_registries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\n").unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert!(config.registries.is_empty());
        assert_eq!(config.search.agent_command, None);
    }

    #[test]
    fn get_value_registries_and_search() {
        let mut config = GlobalConfig::default();
        config.registries.insert("skills-sh".to_string(), RegistryConfig {
            url: "https://skills.sh/api".to_string(),
            default: Some(true),
        });
        config.search.agent_command = Some("claude search {query}".to_string());

        assert_eq!(config.get_value("registries.skills-sh"), Some("https://skills.sh/api".to_string()));
        assert_eq!(config.get_value("search.agent-command"), Some("claude search {query}".to_string()));
    }
}
