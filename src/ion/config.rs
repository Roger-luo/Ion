use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::utils::config_file;

#[derive(Serialize, Deserialize, Debug)]
pub struct Julia {
    pub exename: PathBuf, // the Julia command path
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitHub {
    pub username: String, // GitHub username
    pub token: String,    // GitHub token
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub github: Option<GitHub>,
    pub julia: Option<Julia>,
    pub template: Option<String>, // url to the template registry
    pub env: Option<String>,      // env directory path
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github: None,
            julia: Some(Julia {
                exename: PathBuf::from("julia"),
            }),
            template: None,
            env: Some("env".into()),
        }
    }
}

impl Config {
    pub fn write(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_file()?, content)?;
        Ok(())
    }

    pub fn read() -> Result<Self> {
        let content = std::fs::read_to_string(config_file()?)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}
