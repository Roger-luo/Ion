use ion::TemplateDerive;
use serde_derive::Deserialize;
use super::{components::*, Context};
use super::{Blueprint, RenderResult};
use crate::dirs::template_dir;
use log::debug;

#[derive(Debug, Deserialize, TemplateDerive)]
pub struct Template {
    pub name: String, // name of the template
    pub description: String, // description of the template
    // the following has order of appearance in prompts
    pub project_file: Option<ProjectFile>,
    pub readme: Option<Readme>,
    pub src_dir: Option<SrcDir>,
    pub tests: Option<ProjectTest>,
    pub license: Option<License>,
    pub citation: Option<Citation>,
    pub documenter: Option<Documenter>,
    pub repo: Option<GitRepo>,
}
