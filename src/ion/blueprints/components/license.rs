use crate::blueprints::*;
use crate::utils::components_dir;
use chrono::Datelike;
use dialoguer::Input;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct Info {
    pub year: i32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct License {
    template_dir: Option<PathBuf>,
}

impl Blueprint for License {
    // license is not specified in a template.toml file
    // so we need to prompt the user for a license
    fn prompt(&self, _t: &Template, config: &Config, ctx: &mut Context) -> RenderResult {
        let name = Input::<String>::new()
            .with_prompt("license")
            .allow_empty(false)
            .with_initial_text("MIT")
            .interact_text()
            .expect("prompt failed for license");

        let current_date = chrono::Utc::now();
        let year = Input::<i32>::new()
            .with_prompt("year")
            .allow_empty(false)
            .with_initial_text(current_date.year().to_string())
            .interact_text()
            .expect("prompt failed for year");
        ctx.license = Some(Info { name, year });
        Ok(())
    }

    fn render(&self, _t: &Template, config: &Config, ctx: &Context) -> RenderResult {
        let root = match &self.template_dir {
            Some(dir) => dir.to_owned(),
            None => components_dir()?.join("licenses"),
        };
        let license = ctx.license.as_ref().unwrap().name.to_owned();
        TemplateFile {
            root,
            path: PathBuf::from("."),
            file: license + ".hbs",
        }
        .render(ctx, "LICENSE")
    }
}
