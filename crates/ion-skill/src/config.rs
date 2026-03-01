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
