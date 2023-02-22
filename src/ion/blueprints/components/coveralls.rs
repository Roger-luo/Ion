use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct Coveralls {
    #[serde(default)]
    pub template: Option<String>,
}

impl Blueprint for Coveralls {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(ref template) = self.template {
            template
                .as_template(config)?
                .render(ctx, ".coveralls.yml")?;
        }
        Ok(())
    }
}
