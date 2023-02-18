use crate::blueprints::*;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

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

impl fmt::Display for Codecov {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        if let Some(template) = &self.template {
            writeln!(format_buffer, "CodeCov template: {template}")?;
        } else {
            writeln!(format_buffer, "CodeCov template: None")?;
        }
        Ok(())
    }
}
