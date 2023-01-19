use anyhow::Ok;
use serde_derive::Deserialize;
use crate::blueprints::*;
use crate::commands::pkg::{JuliaCmd, PackageSpec};

#[derive(Debug, Deserialize)]
pub struct Documenter {
    #[serde(default = "Documenter::default_make_jl")]
    make_jl: TemplateFile,
    #[serde(default = "Documenter::default_index_md")]
    index_md: TemplateFile,
    #[serde(default = "Documenter::default_doc_project")]
    doc_project: TemplateFile,
    #[serde(default = "Documenter::default_ignore")]
    ignore: Vec<String>,
}

impl Documenter {
    pub fn default_make_jl() -> TemplateFile {
        TemplateFile::from_str("docs/make.jl.hbs")
    }

    pub fn default_index_md() -> TemplateFile {
        TemplateFile::from_str("docs/src/index.md.hbs")
    }

    pub fn default_doc_project() -> TemplateFile {
        TemplateFile::from_str("docs/src/project.md.hbs")
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
    fn collect(&self, ctx: &mut Context) -> RenderResult {
        for ignore in &self.ignore {
            ctx.ignore(ignore);
        }
        Ok(())
    }

    fn render(&self, ctx: &Context) -> RenderResult {
        self.make_jl.render(ctx, "docs/make.jl")?;
        self.index_md.render(ctx, "docs/src/index.md")?;
        self.doc_project.render(ctx, "docs/src/project.md")?;
        if let Err(e) = format!(
            "using Pkg; Pkg.develop({})",
            PackageSpec::from_path(&project_dir(ctx))
        ).as_julia_script() {
            return Err(e.error.unwrap());
        }
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
