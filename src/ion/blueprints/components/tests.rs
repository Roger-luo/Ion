use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct ProjectTest {
    #[serde(default = "ProjectTest::default_template")]
    template: String,
    #[serde(default = "ProjectTest::default_project")]
    project: String,
}

impl ProjectTest {
    pub fn default_template() -> String {
        "test/runtests.jl.hbs".into()
    }

    pub fn default_project() -> String {
        "test/Project.toml.hbs".into()
    }
}

impl Blueprint for ProjectTest {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        self.template
            .as_template(config)?
            .render(ctx, "runtests.jl")?;
        self.project
            .as_template(config)?
            .render(ctx, "Project.toml")
    }
}
