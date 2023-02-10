use crate::blueprints::*;
use crate::spec::Author;
use crate::utils::find::julia_version;
use serde_derive::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct Git {
    pub user: String,
    pub email: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Julia {
    pub version: String,
    pub compat: String,
}

impl Julia {
    pub fn new(config: &Config) -> Self {
        let version = match julia_version(config) {
            Ok(version) => version,
            Err(_) => node_semver::Version::parse("1.6.0").unwrap(),
        };

        let compat = format!("{}.{}", version.major, version.minor);
        Julia {
            version: version.to_string(),
            compat,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub uuid: Option<String>,
    pub version: Option<String>,
    pub authors: Vec<Author>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub homepage: Option<String>,
    pub git: Option<Git>,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        let git = if let Ok((user, email)) = git_get_user() {
            Some(Git { user, email })
        } else {
            None
        };

        Project {
            name,
            path,
            uuid: None,
            version: None,
            authors: Vec::new(),
            description: None,
            keywords: Vec::new(),
            homepage: None,
            git,
        }
    }
}
