use serde_derive::Deserialize;
use crate::blueprints::*;

#[derive(Debug, Deserialize)]
pub struct SrcDir{
    #[serde(default = "SrcDir::default_template")]
    template: TemplateFile,
}

impl SrcDir{
    pub fn default_template() -> TemplateFile{
        TemplateFile::from_str("src/module.jl.hbs")
    }
}

impl Blueprint for SrcDir {
    fn render(&self, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "module.jl")
    }
}
