use crate::bump::VersionBumpHandler;
use crate::summon::JuliaRegistrator;
use crate::utils::current_root_project;
use crate::VersionSpec;
use anyhow::{format_err, Result};
use node_semver::Version;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JuliaProject {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub authors: Option<Vec<String>>,
    pub version: Option<Version>,
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

    pub fn write(&self, path: &PathBuf) -> Result<()> {
        let toml = toml::to_string(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JuliaProjectFile {
    pub toml: PathBuf,
    pub path: PathBuf,
    pub project: JuliaProject,
}

impl JuliaProjectFile {
    pub fn root_project(path: PathBuf) -> Result<Self> {
        let (project, toml) = match current_root_project(path) {
            Some((project, path)) => (project, path),
            None => return Err(format_err!("No Project.toml found")),
        };

        let path_to_project = match toml.parent() {
            Some(path) => path.to_path_buf().canonicalize()?,
            None => return Err(format_err!("No parent directory found")),
        };

        Ok(JuliaProjectFile {
            path: path_to_project,
            toml,
            project,
        })
    }

    pub fn get_version(&self) -> Result<Version> {
        match &self.project.version {
            Some(version) => Ok(version.to_owned()),
            None => Err(format_err!("No version found")),
        }
    }

    pub fn update_version(&mut self, version: &Version) -> &mut Self {
        self.project.update_version(version);
        self
    }

    pub fn write(&self) -> Result<()> {
        self.project.write(&self.toml)?;
        Ok(())
    }

    pub fn bump(&self, version_spec: VersionSpec) -> VersionBumpHandler {
        VersionBumpHandler::new(self.clone(), version_spec)
    }

    pub fn summon(&self) -> Result<JuliaRegistrator> {
        JuliaRegistrator::from_project(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use node_semver::Version;

    #[test]
    fn test_update_version() -> Result<(), Error> {
        let mut project = JuliaProject {
            name: Some("Test".to_string()),
            uuid: Some("12345678".to_string()),
            authors: Some(vec!["Test".to_string()]),
            version: Some(Version::parse("0.1.0")?),
            description: Some("Test".to_string()),
            license: Some("MIT".to_string()),
            deps: BTreeMap::new(),
            compat: None,
            extras: None,
            targets: None,
        };
        project.update_version(&Version::parse("0.2.0")?);
        assert_eq!(project.version, Some(Version::parse("0.2.0")?));
        Ok(())
    }
}
