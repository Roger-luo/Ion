use serde_derive::{Deserialize, Serialize};
use crate::blueprints::*;

#[derive(Debug, Serialize, Clone)]
pub struct Info {
    pub documenter: bool,
    pub codecov: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct GitHub {
    pub ci: Option<CI>,
    pub tagbot: Option<TagBot>,
    pub compat_helper: Option<CompatHelper>,
}

impl Blueprint for GitHub {
    fn collect(&self, t: &Template, ctx: &mut Context) -> RenderResult {
        ctx.github = Some(Info {
            documenter: t.documenter.is_some(),
            codecov: t.codecov.is_some(),
        });
        Ok(())
    }

    fn render(&self, t: &Template, ctx: &Context) -> RenderResult {
        if let Some(ref ci) = self.ci {
            ci.render(t, ctx)?;
        }
        if let Some(ref tagbot) = self.tagbot {
            tagbot.render(t, ctx)?;
        }
        if let Some(ref compat_helper) = self.compat_helper {
            compat_helper.render(t, ctx)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CI {
    pub template: TemplateFile,
    pub arch: Vec<String>,
    pub os: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TagBot {
    pub template: TemplateFile,
}

#[derive(Debug, Deserialize)]
pub struct CompatHelper {
    pub template: TemplateFile,
}

impl Default for CI {
    fn default() -> Self {
        CI {
            template: TemplateFile::from_str("github/workflows/CI.yml.hbs"),
            arch: vec!["x86".to_string(), "x64".to_string()],
            os: vec!["ubuntu-latest".to_string(), "windows-latest".to_string(), "macos-latest".to_string()],
        }
    }
}

impl Default for TagBot
{
    fn default() -> Self {
        TagBot {
            template: TemplateFile::from_str("github/workflows/TagBot.yml.hbs"),
        }
    }
}

impl Default for CompatHelper
{
    fn default() -> Self {
        CompatHelper {
            template: TemplateFile::from_str("github/workflows/CompatHelper.yml.hbs"),
        }
    }
}

impl Blueprint for CI {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "CI.yml")
    }
}

impl Blueprint for TagBot {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "TagBot.yml")
    }
}

impl Blueprint for CompatHelper {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "CompatHelper.yml")
    }
}
