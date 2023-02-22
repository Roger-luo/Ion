use crate::blueprints::*;
use log::debug;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

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
    fn collect(&self, t: &Template, _config: &Config, ctx: &mut Context) -> RenderResult {
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

    fn render(&self, t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        if let Some(ref ci) = self.ci {
            ci.render(t, config, ctx)?;
        }
        if let Some(ref tagbot) = self.tagbot {
            tagbot.render(t, config, ctx)?;
        }
        if let Some(ref compat_helper) = self.compat_helper {
            compat_helper.render(t, config, ctx)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CI {
    #[serde(default = "CI::default_template")]
    pub template: String,
    pub arch: Vec<String>,
    pub os: Vec<String>,
}

impl CI {
    fn default_template() -> String {
        ".github/workflows/CI.yml.hbs".into()
    }
}

#[derive(Debug, Deserialize)]
pub struct TagBot {
    #[serde(default = "TagBot::default_template")]
    pub template: String,
}

impl TagBot {
    fn default_template() -> String {
        ".github/workflows/TagBot.yml".into()
    }
}

#[derive(Debug, Deserialize)]
pub struct CompatHelper {
    #[serde(default = "CompatHelper::default_template")]
    pub template: String,
}

impl CompatHelper {
    fn default_template() -> String {
        ".github/workflows/CompatHelper.yml".into()
    }
}

impl Default for CI {
    fn default() -> Self {
        CI {
            template: ".github/workflows/CI.yml.hbs".into(),
            arch: vec!["x86".to_string(), "x64".to_string()],
            os: vec![
                "ubuntu-latest".to_string(),
                "windows-latest".to_string(),
                "macos-latest".to_string(),
            ],
        }
    }
}

impl Default for TagBot {
    fn default() -> Self {
        TagBot {
            template: ".github/workflows/TagBot.yml".into(),
        }
    }
}

impl Default for CompatHelper {
    fn default() -> Self {
        CompatHelper {
            template: ".github/workflows/CompatHelper.yml".into(),
        }
    }
}

impl Blueprint for CI {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        debug!("rendering CI.yml: {:#?}", ctx.github);
        self.template.as_template(config)?.render(ctx, "CI.yml")
    }
}

impl Blueprint for TagBot {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        self.template.as_template(config)?.copy(ctx, "TagBot.yml")
    }
}

impl Blueprint for CompatHelper {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        self.template
            .as_template(config)?
            .copy(ctx, "CompatHelper.yml")
    }
}

impl fmt::Display for GitHub {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ci) = &self.ci {
            write!(format_buffer, "{:#?}", ci)?;
        }
        if let Some(tagbot) = &self.tagbot {
            write!(format_buffer, "{:#?}", tagbot)?;
        }
        if let Some(compat_helper) = &self.compat_helper {
            write!(format_buffer, "{:#?}", compat_helper)?;
        }
        Ok(())
    }
}

impl fmt::Display for CI {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        write!(format_buffer, "{:#?}", self.template)?;
        write!(format_buffer, "{:#?}", self.arch)?;
        write!(format_buffer, "{:#?}", self.os)?;
        Ok(())
    }
}

impl fmt::Display for TagBot {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        writeln!(format_buffer, "{:#?}", self.template)?;
        Ok(())
    }
}

impl fmt::Display for CompatHelper {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        writeln!(format_buffer, "{:#?}", self.template)?;
        Ok(())
    }
}
