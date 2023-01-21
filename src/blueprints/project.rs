use std::path::PathBuf;
use serde_derive::Serialize;
use crate::blueprints::*;

#[derive(Debug, Serialize, Clone)]
pub struct Git {
    pub user: String,
    pub email: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Author {
    pub firstname: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub affiliation: Option<String>,
    pub orcid: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Julia {
    pub version: String,
    pub compat: String,
}

impl Default for Julia {
    fn default() -> Self {
        let julia_version_str = match julia_version() {
            Ok(version_str) => {
                version_str[14..].to_string()
            },
            Err(_) => {
                "1.6.0".to_string()
            }
        };

        let version = match node_semver::Version::parse(&julia_version_str) {
            Ok(version) => version,
            Err(_) => {
                node_semver::Version::parse("1.6.0").unwrap()
            }
        };

        let compat = format!("{}.{}", version.major, version.minor);
        Julia {
            version: julia_version_str,
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
            Some(Git {
                user,
                email,
            })
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
