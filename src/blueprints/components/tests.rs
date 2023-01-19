use serde_derive::Deserialize;
use crate::blueprints::*;

#[derive(Debug, Deserialize)]
pub struct ProjectTest {
    #[serde(default = "ProjectTest::default_template")]
    template: TemplateFile,
    #[serde(default = "ProjectTest::default_project")]
    project: TemplateFile,
}

impl ProjectTest {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_str("tests/runtests.jl")
    }

    pub fn default_project() -> TemplateFile {
        TemplateFile::from_str("tests/Project.toml")
    }
}

impl Blueprint for ProjectTest {
    fn render(&self, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "runtests.jl")?;
        self.project.render(ctx, "Project.toml")
    }
}
