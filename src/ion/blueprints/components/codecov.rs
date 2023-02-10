use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct Codecov {
    #[serde(default)]
    pub template: Option<String>,
}

impl Blueprint for Codecov {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(ref template) = self.template {
            template.as_template(config)?.render(ctx, ".codecov.yml")?;
        }
        Ok(())
    }
}
