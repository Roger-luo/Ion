use serde_derive::{Serialize, Deserialize};
use crate::blueprints::*;

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct ProjectTest {
    #[serde(default = "ProjectTest::default_template")]
    template: TemplateFile,
    #[serde(default = "ProjectTest::default_project")]
    project: TemplateFile,
}

impl ProjectTest {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_str("tests/runtests.jl.hbs")
    }

    pub fn default_project() -> TemplateFile {
        TemplateFile::from_str("tests/Project.toml.hbs")
    }
}

impl Blueprint for ProjectTest {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "runtests.jl")?;
        self.project.render(ctx, "Project.toml")
    }
}
