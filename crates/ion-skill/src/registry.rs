use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoEntry {
    pub url: String,
    #[serde(default)]
    pub projects: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub repos: BTreeMap<String, RepoEntry>,
}

impl Registry {
    /// Returns the path to the global registry file.
    pub fn registry_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("ion").join("registry.toml"))
    }

    /// Load the global registry. Returns empty registry if file doesn't exist.
    pub fn load() -> Result<Self> {
        match Self::registry_path() {
            Some(path) => Self::load_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load registry from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        crate::load_toml_or_default(path)
    }

    /// Save registry to the default path.
    pub fn save(&self) -> Result<()> {
        match Self::registry_path() {
            Some(path) => self.save_to(&path),
            None => Ok(()),
        }
    }

    /// Save registry to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Manifest(format!("Failed to serialize registry: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }

    /// Register that a project uses a specific repo.
    pub fn register(&mut self, repo_hash: &str, url: &str, project_dir: &str) {
        let entry = self
            .repos
            .entry(repo_hash.to_string())
            .or_insert_with(|| RepoEntry {
                url: url.to_string(),
                projects: Vec::new(),
            });
        if !entry.projects.contains(&project_dir.to_string()) {
            entry.projects.push(project_dir.to_string());
            entry.projects.sort();
        }
    }

    /// Unregister a project from a specific repo.
    pub fn unregister(&mut self, repo_hash: &str, project_dir: &str) {
        if let Some(entry) = self.repos.get_mut(repo_hash) {
            entry.projects.retain(|p| p != project_dir);
        }
    }

    /// Remove repos with no remaining projects. Returns list of removed repo hashes.
    pub fn cleanup_stale(&mut self) -> Vec<(String, String)> {
        let mut removed = Vec::new();
        self.repos.retain(|hash, entry| {
            // Remove projects whose directories no longer exist
            entry.projects.retain(|p| Path::new(p).exists());
            if entry.projects.is_empty() {
                removed.push((hash.clone(), entry.url.clone()));
                false
            } else {
                true
            }
        });
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister() {
        let mut registry = Registry::default();

        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/home/user/project",
        );
        assert_eq!(registry.repos["abc123"].projects.len(), 1);

        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/home/user/project",
        );
        assert_eq!(
            registry.repos["abc123"].projects.len(),
            1,
            "should be idempotent"
        );

        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/home/user/other",
        );
        assert_eq!(registry.repos["abc123"].projects.len(), 2);

        registry.unregister("abc123", "/home/user/project");
        assert_eq!(registry.repos["abc123"].projects.len(), 1);
    }

    #[test]
    fn cleanup_removes_nonexistent_projects() {
        let mut registry = Registry::default();
        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/nonexistent/path/1",
        );
        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/nonexistent/path/2",
        );

        let removed = registry.cleanup_stale();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "abc123");
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn cleanup_keeps_existing_projects() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::default();
        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            &dir.path().display().to_string(),
        );
        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/nonexistent/path",
        );

        let removed = registry.cleanup_stale();

        assert!(removed.is_empty());
        assert_eq!(registry.repos["abc123"].projects.len(), 1);
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.toml");

        let mut registry = Registry::default();
        registry.register(
            "abc123",
            "https://github.com/org/repo.git",
            "/home/user/project",
        );

        registry.save_to(&path).unwrap();

        let loaded = Registry::load_from(&path).unwrap();
        assert_eq!(loaded.repos.len(), 1);
        assert_eq!(
            loaded.repos["abc123"].url,
            "https://github.com/org/repo.git"
        );
        assert_eq!(
            loaded.repos["abc123"].projects,
            vec!["/home/user/project"]
        );
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let registry = Registry::load_from(Path::new("/nonexistent/registry.toml")).unwrap();
        assert!(registry.repos.is_empty());
    }
}
