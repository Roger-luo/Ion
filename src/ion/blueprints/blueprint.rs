use anyhow::Error;
use super::{Context, Template};

pub type RenderResult = Result<(), Error>;

pub trait Blueprint {
    fn render(&self, t: &Template, ctx: &Context) -> RenderResult;
    fn collect(&self, _t: &Template, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn prompt(&self, _t: &Template, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    // propagate collected or propmted data to other meta
    // information fields
    fn propagate(&self, _t: &Template, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn post_render(&self, _t: &Template, _ctx: &Context) -> RenderResult {
        Ok(())
    }
    fn validate(&self, _t: &Template, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl Blueprint for String {
    fn render(&self, _t: &Template, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl<T: Blueprint> Blueprint for Option<T> {
    fn render(&self, t: &Template, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.render(t, ctx)?;
        }
        Ok(())
    }

    fn prompt(&self, t: &Template, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.prompt(t, ctx)?;
        }
        Ok(())        
    }

    fn collect(&self, t: &Template, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.collect(t, ctx)?;
        }
        Ok(())        
    }

    fn post_render(&self, t: &Template, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.post_render(t, ctx)?;
        }
        Ok(())
    }

    fn propagate(&self, t: &Template, ctx: &mut Context) -> RenderResult {
        if let Some(bp) = self {
            bp.propagate(t, ctx)?;
        }
        Ok(())
    }

    fn validate(&self, t: &Template, ctx: &Context) -> RenderResult {
        if let Some(bp) = self {
            bp.validate(t, ctx)?;
        }
        Ok(())
    }
}
