use anyhow::Error;
use super::Context;

pub type RenderResult = Result<(), Error>;

pub trait Blueprint {
    fn render(&self, ctx: &Context) -> RenderResult;
    fn collect(&self, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn prompt(&self, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    // propagate collected or propmted data to other meta
    // information fields
    fn propagate(&self, _ctx: &mut Context) -> RenderResult {
        Ok(())
    }
    fn post_render(&self, _ctx: &Context) -> RenderResult {
        Ok(())
    }
    fn validate(&self, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl Blueprint for String {
    fn render(&self, _ctx: &Context) -> RenderResult {
        Ok(())
    }
}

impl<T: Blueprint> Blueprint for Option<T> {
    fn render(&self, ctx: &Context) -> RenderResult {
        if let Some(t) = self {
            t.render(ctx)?;
        }
        Ok(())
    }

    fn prompt(&self, ctx: &mut Context) -> RenderResult {
        if let Some(t) = self {
            t.prompt(ctx)?;
        }
        Ok(())        
    }

    fn collect(&self, ctx: &mut Context) -> RenderResult {
        if let Some(t) = self {
            t.collect(ctx)?;
        }
        Ok(())        
    }
}
