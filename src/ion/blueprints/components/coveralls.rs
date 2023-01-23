use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct Coveralls {
    #[serde(default)]
    pub template: Option<TemplateFile>,
}

impl Blueprint for Coveralls {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        if let Some(ref template) = self.template {
            template.render(ctx, ".coveralls.yml")?;
        }
        Ok(())
    }
}
