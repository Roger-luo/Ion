use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct SrcDir {
    #[serde(default = "SrcDir::default_template")]
    template: TemplateFile,
}

impl SrcDir {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_path_str("src/module.jl.hbs")
    }
}

impl Blueprint for SrcDir {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        let module = ctx.project.name.to_owned();
        self.template.render(ctx, format!("{}.jl", module).as_str())
    }
}
