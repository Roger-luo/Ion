use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    pub version: Option<String>,
    pub uuid: String,
    pub deps: Option<Vec<String>>,
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
