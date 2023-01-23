use anyhow::Error;
use node_semver::Version;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JuliaProject {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub version: Option<Version>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub deps: BTreeMap<String, String>,
    pub compat: Option<BTreeMap<String, String>>,
    pub extras: Option<BTreeMap<String, String>>,
    pub targets: Option<BTreeMap<String, Vec<String>>>,
}

impl JuliaProject {
    pub fn update_version(&mut self, version: &Version) -> &mut Self {
        self.version = Some(version.to_owned());
        self
    }

    pub fn write(&self, path: &PathBuf) -> Result<(), Error> {
        let toml = toml::to_string(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}
