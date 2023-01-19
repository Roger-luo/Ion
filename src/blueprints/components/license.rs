use std::path::PathBuf;
use serde_derive::Deserialize;
use crate::blueprints::*;
use crate::dirs::components_dir;

#[derive(Debug, Deserialize)]
pub struct License {
    #[serde(default = "License::default_template_dir")]
    template_dir: PathBuf,
    name: String,
}

impl License {
    pub fn default_template_dir() -> PathBuf {
        components_dir().join("license")
    }
}

impl Blueprint for License {
    fn render(&self, ctx: &Context) -> RenderResult {
        TemplateFile::from_path(self.template_dir.join(&self.name))
            .render(ctx, "LICENSE")
    }
}
