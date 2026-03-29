//! Parsing for `template.toml` manifest files.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::Error;

/// A parsed `template.toml` manifest.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TemplateManifest {
    /// Declared template variables.
    pub variables: HashMap<String, VariableDecl>,
    /// File configuration: optional files, mappings, symlinks.
    pub files: FilesConfig,
}

/// Declaration of a single template variable.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct VariableDecl {
    /// Human-readable description of the variable.
    pub description: Option<String>,
    /// Default value. If `None`, the variable is required.
    pub default: Option<String>,
}

/// File-related configuration from `template.toml`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct FilesConfig {
    /// Glob patterns for files excluded by default (opt-in via `.include()`).
    pub optional: Vec<String>,
    /// Source path (in template dir) → destination path (rendered).
    pub mappings: HashMap<String, String>,
    /// Symlink path (rendered) → symlink target (rendered).
    pub symlinks: HashMap<String, String>,
}

impl TemplateManifest {
    /// Parse `template.toml` from the given template directory.
    pub fn from_dir(dir: &Path) -> Result<Self, Error> {
        let path = dir.join("template.toml");
        let content = std::fs::read_to_string(&path)
            .map_err(|_| Error::TemplateNotFound { path: path.clone() })?;
        let manifest: TemplateManifest =
            toml::from_str(&content).map_err(|source| Error::ManifestParse {
                path: path.clone(),
                source,
            })?;
        Ok(manifest)
    }
}
