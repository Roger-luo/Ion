use std::collections::HashMap;
use serde_derive::Deserialize;
use crate::blueprints::*;

#[derive(Debug, Deserialize)]
pub struct Citation {
    template: TemplateFile,
    readme: bool,
}

impl Citation {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_str("./CITATION.cff.hbs")
    }
}

impl Blueprint for Citation {
    fn collect(&self, ctx: &mut Context) -> RenderResult {
        if !ctx.meta.contains_key("citation") {
            let mut citation = HashMap::new();
            citation.insert("readme".to_string(), Meta::Bool(self.readme));
            ctx.meta.insert("citation".to_string(), Meta::Object(citation));
        }
        Ok(())
    }

    fn prompt(&self, ctx: &mut Context) -> RenderResult {
        prompt_for_authors(ctx)
    }

    fn render(&self, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "CITATION.cff")
    }
}
