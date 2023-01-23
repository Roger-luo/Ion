use super::badge::Badge;
use crate::blueprints::*;
use anyhow::Error;
use clap::ArgMatches;
use dialoguer::Input;
use log::debug;
use serde_derive::Serialize;
use std::path::PathBuf;

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
pub struct Citation {
    pub readme: bool,
    pub title: String,
    pub authors: Vec<Author>,
    pub year: i32,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub number: Option<String>,
    pub pages: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Julia {
    pub version: String,
    pub compat: String,
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
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        Project {
            name,
            path,
            uuid: None,
            version: None,
            authors: Vec::new(),
            description: None,
            keywords: Vec::new(),
            homepage: None,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Git {
    pub user: String,
    pub email: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Repository {
    pub url: String,
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct License {
    pub year: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CI {
    pub documenter: bool,
    pub codecov: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct Context {
    pub prompt: bool,
    pub julia: Julia,
    pub project: Project,
    pub git: Option<Git>,
    pub badges: Vec<Badge>,
    pub license: Option<License>,
    pub repository: Option<Repository>,
    pub citation: Option<Citation>,
    pub ci: Option<CI>,
    pub ignore: Vec<String>,
}

impl Context {
    pub fn ignore(&mut self, path: &str) -> &mut Self {
        self.ignore.push(path.to_string());
        self
    }

    pub fn from_matches(matches: &ArgMatches) -> Result<Self, Error> {
        let prompt = !matches.get_flag("no-interactive");
        let package = match matches.get_one::<String>("name") {
            Some(name) => name.to_owned(),
            None => {
                if prompt {
                    Input::<String>::new()
                        .with_prompt("name of the project")
                        .allow_empty(false)
                        .interact_text()
                        .expect("error")
                } else {
                    return Err(anyhow::format_err!("No name provided."));
                }
            }
        };
        let path = std::env::current_dir().unwrap().join(&package);

        debug!("path: {}", path.display());
        if path.is_dir() {
            if matches.get_flag("force") {
                debug!("removing existing directory: {}", path.display());
                std::fs::remove_dir_all(&path)?;
            } else {
                return Err(anyhow::format_err!(
                    "project already exists:{}",
                    path.display()
                ));
            }
        }
        std::fs::create_dir_all(&path).unwrap();

        let julia_version_str = julia_version()?[14..].to_string();
        let version = node_semver::Version::parse(&julia_version_str)?;
        let compat = format!("{}.{}", version.major, version.minor);
        let julia = Julia {
            version: julia_version_str,
            compat,
        };

        let git = if let Ok((user, email)) = git_get_user() {
            Some(Git { user, email })
        } else {
            None
        };

        Ok(Context {
            prompt,
            julia,
            project: Project::new(package, path),
            git,
            license: None,
            badges: Vec::new(),
            repository: None,
            citation: None,
            ignore: Vec::new(),
            ci: None,
        })
    }
}
