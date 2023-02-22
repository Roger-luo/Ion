use crate::blueprints::*;
use crate::spec::Author;
use dialoguer::Confirm;
use julia_semver::Version;
use log::debug;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct ProjectFile {
    #[serde(default = "ProjectFile::default_template")]
    template: String,
    #[serde(default = "ProjectFile::default_version")]
    version: Version,
}

impl ProjectFile {
    pub fn default_template() -> String {
        "./Project.toml.hbs".into()
    }

    pub fn default_version() -> Version {
        Version::parse("0.1.0").unwrap()
    }
}

impl Blueprint for ProjectFile {
    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        self.template
            .as_template(config)?
            .render(ctx, "Project.toml")
    }

    fn prompt(&self, _t: &Template, _config: &Config, ctx: &mut Context) -> RenderResult {
        let msg = if !ctx.project.authors.is_empty() {
            format!("authors (default: {})", ctx.project.authors[0].firstname)
        } else {
            "authors".to_string()
        };

        if !Confirm::new()
            .with_prompt(msg.as_str())
            .default(true)
            .interact()?
        {
            ctx.project.authors = prompt_for_authors()?;
        }
        Ok(())
    }

    fn collect(&self, _t: &Template, _config: &Config, ctx: &mut Context) -> RenderResult {
        ctx.project.version = Some(self.version.to_string());
        ctx.project.uuid = Some(Uuid::new_v4().to_string());

        if let Some(repo) = &mut ctx.repo {
            repo.ignore.push("/Manifest.toml".to_string());
        }

        // if no prompt, but git is setup, use git user.name/email as author
        debug!("git is setup, use git user.name/email as an author");
        if let Some(Git { user, email }) = &ctx.project.git {
            ctx.project.authors = vec![Author {
                firstname: user.to_owned(),
                lastname: None,
                email: Some(email.to_owned()),
                url: None,
                affiliation: None,
                orcid: None,
            }];
        }
        Ok(())
    }
}
