use super::{Context, Template};
use crate::config::Config;
use anyhow::Error;

pub type RenderResult = Result<(), Error>;

pub trait Blueprint {
    fn render(&self, t: &Template, config: &Config, ctx: &Context) -> RenderResult;
    fn collect(&self, _t: &Template, _config: &Config, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn prompt(&self, _t: &Template, _config: &Config, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    // propagate collected or propmted data to other meta
    // information fields
    fn propagate(&self, _t: &Template, _config: &Config, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn post_render(&self, _t: &Template, _config: &Config, _ctx: &Context) -> RenderResult {
        Ok(())
    }
    fn validate(&self, _t: &Template, _config: &Config, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl Blueprint for String {
    fn render(&self, _t: &Template, _config: &Config, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl<T: Blueprint> Blueprint for Option<T> {
    fn render(&self, t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.render(t, config, ctx)?;
        }
        Ok(())
    }

    fn prompt(&self, t: &Template, config: &Config, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.prompt(t, config, ctx)?;
        }
        Ok(())
    }

    fn collect(&self, t: &Template, config: &Config, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.collect(t, config, ctx)?;
        }
        Ok(())
    }

    fn post_render(&self, t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.post_render(t, config, ctx)?;
        }
        Ok(())
    }

    fn propagate(&self, t: &Template, config: &Config, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.propagate(t, config, ctx)?;
        }
        Ok(())
    }

    fn validate(&self, t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.validate(t, config, ctx)?;
        }
        Ok(())
    }
}
