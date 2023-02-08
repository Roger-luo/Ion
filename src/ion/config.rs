use serde_derive::{Deserialize, Serialize};

use std::path::PathBuf;

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
