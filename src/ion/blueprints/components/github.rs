use log::debug;
use serde_derive::{Deserialize, Serialize};
use crate::blueprints::*;

#[derive(Debug, Serialize, Clone)]
pub struct Info {
    pub documenter: bool,
    pub codecov: bool,
    pub coveralls: bool,
    pub arch: Vec<String>,
    pub os: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GitHub {
    pub ci: Option<CI>,
    pub tagbot: Option<TagBot>,
    pub compat_helper: Option<CompatHelper>,
}

impl Blueprint for GitHub {
    fn collect(&self, t: &Template, ctx: &mut Context) -> RenderResult {
        let arch = match self.ci {
            Some(ref ci) => ci.arch.clone(),
            None => Vec::<String>::new(),
        };
        let os = match self.ci {
            Some(ref ci) => ci.os.clone(),
            None => Vec::<String>::new(),
        };

        ctx.github = Some(Info {
            documenter: t.documenter.is_some(),
            codecov: t.codecov.is_some(),
            coveralls: t.coveralls.is_some(),
            arch,
            os,
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
    #[serde(default="CI::default_template")]
    pub template: TemplateFile,
    pub arch: Vec<String>,
    pub os: Vec<String>,
}

impl CI {
    fn default_template() -> TemplateFile {
        TemplateFile::from_str("github/workflows/CI.yml.hbs")
    }
}

#[derive(Debug, Deserialize)]
pub struct TagBot {
    #[serde(default="TagBot::default_template")]
    pub template: TemplateFile,
}

impl TagBot {
    fn default_template() -> TemplateFile {
        TemplateFile::from_str("github/workflows/TagBot.yml")
    }
}

#[derive(Debug, Deserialize)]
pub struct CompatHelper {
    #[serde(default="CompatHelper::default_template")]
    pub template: TemplateFile,
}

impl CompatHelper {
    fn default_template() -> TemplateFile {
        TemplateFile::from_str("github/workflows/CompatHelper.yml")
    }
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
            template: TemplateFile::from_str("github/workflows/TagBot.yml"),
        }
    }
}

impl Default for CompatHelper
{
    fn default() -> Self {
        CompatHelper {
            template: TemplateFile::from_str("github/workflows/CompatHelper.yml"),
        }
    }
}

impl Blueprint for CI {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        debug!("rendering CI.yml: {:#?}", ctx.github);
        self.template.render(ctx, "CI.yml")
    }
}

impl Blueprint for TagBot {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.copy(ctx, "TagBot.yml")
    }
}

impl Blueprint for CompatHelper {
    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.copy(ctx, "CompatHelper.yml")
    }
}
