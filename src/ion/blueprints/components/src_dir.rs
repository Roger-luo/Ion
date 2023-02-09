use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct SrcDir {
    #[serde(default = "SrcDir::default_template")]
    template: String,
}

impl SrcDir {
    pub fn default_template() -> String {
        "src/module.jl.hbs".into()
    }
}

impl Blueprint for SrcDir {
    fn render(&self, _t: &Template, _config: &Config, ctx: &Context) -> RenderResult {
        let module = ctx.project.name.to_owned();
        self.template
            .as_template()?
            .render(ctx, format!("{module}.jl").as_str())
    }
}
