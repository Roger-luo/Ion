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
        if segments.len() == 2 {
            if let Some(expanded) = self.sources.get(segments[0]) {
                return format!("{}/{}", expanded, segments[1]);
            }
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
}
