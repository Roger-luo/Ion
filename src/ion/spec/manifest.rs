use anyhow::Result;
use node_semver::Version;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    pub version: Option<Version>,
    pub uuid: String,
    pub deps: Option<Vec<String>>,
    #[serde(rename = "git-tree-sha1")]
    pub git_tree_sha1: Option<String>,
    #[serde(rename = "repo-rev")]
    pub repo_rev: Option<String>,
    #[serde(rename = "repo-url")]
    pub repo_url: Option<String>,
    #[serde(rename = "repo-subdir")]
    pub repo_subdir: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub julia_version: String,
    pub manifest_format: String,
    pub project_hash: String,
    pub deps: BTreeMap<String, Vec<PackageInfo>>,
}

impl Manifest {
    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let path = path.as_ref();
        let manifest = std::fs::read_to_string(path)?;
        let manifest: Manifest = toml::from_str(&manifest)?;
        Ok(manifest)
    }
}
