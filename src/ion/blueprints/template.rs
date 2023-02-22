use super::badge::Badge;
use super::components::*;
use super::Blueprint;
use super::{Julia, Project};

use ion_derive::Template;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Template)]
pub struct Template {
    pub name: String,        // name of the template
    pub description: String, // description of the template
    // the following has order of appearance in prompts
    pub repo: Option<GitRepo>,
    pub project_file: Option<ProjectFile>,
    pub readme: Option<Readme>,
    pub src_dir: Option<SrcDir>,
    pub tests: Option<ProjectTest>,
    pub license: Option<License>,
    pub citation: Option<Citation>,
    pub documenter: Option<Documenter>,
    pub codecov: Option<Codecov>,
    pub coveralls: Option<Coveralls>,
    pub github: Option<GitHub>,
}
