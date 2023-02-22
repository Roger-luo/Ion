use crate::blueprints::*;
use crate::utils::*;
use crate::PackageSpec;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Clone)]
pub struct Info;

#[derive(Debug, Deserialize)]
pub struct Documenter {
    #[serde(default = "Documenter::default_make_jl")]
    make_jl: String,
    #[serde(default = "Documenter::default_index_md")]
    index_md: String,
    #[serde(default = "Documenter::default_doc_project")]
    doc_project: String,
    #[serde(default = "Documenter::default_ignore")]
    ignore: Vec<String>,
}

impl Documenter {
    pub fn default_make_jl() -> String {
        "docs/make.jl.hbs".into()
    }

    pub fn default_index_md() -> String {
        "docs/src/index.md.hbs".into()
    }

    pub fn default_doc_project() -> String {
        "docs/Project.toml.hbs".into()
    }

    pub fn default_ignore() -> Vec<String> {
        vec![
            "/docs/build/".to_string(),
            "/docs/Manifest.toml".to_string(),
            "/docs/src/assets/main.css".to_string(),
            "/docs/src/assets/indigo.css".to_string(),
        ]
    }
}

impl Blueprint for Documenter {
    fn collect(&self, _t: &Template, _config: &Config, ctx: &mut Context) -> RenderResult {
        for ignore in &self.ignore {
            if let Some(repo) = &mut ctx.repo {
                repo.ignore.push(ignore.to_owned());
            }
        }
        Ok(())
    }

    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        self.make_jl.as_template(config)?.render(ctx, "make.jl")?;
        self.index_md.as_template(config)?.render(ctx, "index.md")?;
        self.doc_project
            .as_template(config)?
            .render(ctx, "Project.toml")?;

        format!(
            "using Pkg; Pkg.develop({})",
            PackageSpec::from_path(&ctx.project.path)
        )
        .julia_exec_project_quiet(config, "docs")?;
        Ok(())
    }
}

impl Badgeable for Documenter {
    fn badge(&self) -> Badge {
        Badge {
            hover: "doc".to_string(),
            image: "https://img.shields.io/badge/docs-stable-blue.svg".to_string(),
            link: "https://JuliaDocs.github.io/Documenter.jl/stable".to_string(),
        }
    }
}

impl fmt::Display for Documenter {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        write!(format_buffer, "{:#?}", self.make_jl)?;
        write!(format_buffer, "{:#?}", self.index_md)?;
        write!(format_buffer, "{:#?}", self.doc_project)?;
        write!(format_buffer, "{:#?}", self.ignore)?;

        Ok(())
    }
}
