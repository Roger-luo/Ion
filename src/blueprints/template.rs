use ion::Blueprint;
use serde_derive::Deserialize;
use super::{components::*, Context};
use super::{Blueprint, RenderResult};
use crate::dirs::template_dir;

#[derive(Debug, Deserialize, Blueprint)]
pub struct Template {
    pub name: String, // name of the template
    pub description: String, // description of the template
    pub citation: Option<Citation>,
    pub documenter: Option<Documenter>,
    pub license: Option<License>,
    pub readme: Option<Readme>,
    pub repo: Option<GitRepo>,
    pub src_dir: Option<SrcDir>,
    pub tests: Option<ProjectTest>,
}

impl Template {
    pub fn load(name: &String) -> Template {
        let mut template = template_dir();
        template.push(name);
        template.push("template.toml");

        assert!(template.is_file(), "Template file not found: {}", template.display());
        let template : Template = toml::from_str(
            &std::fs::read_to_string(template).unwrap()).unwrap();
        template
    }
}
