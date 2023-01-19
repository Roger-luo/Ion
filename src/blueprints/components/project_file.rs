use node_semver::Version;
use serde_derive::Deserialize;
use crate::blueprints::*;

#[derive(Debug, Deserialize)]
pub struct ProjectFile {
    #[serde(default = "ProjectFile::default_template")]
    template: TemplateFile,
    #[serde(default = "ProjectFile::default_version")]
    version: Version,
}

impl ProjectFile {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_str("./Project.toml.hbs")
    }

    pub fn default_version() -> Version {
        Version::parse("0.1.0").unwrap()
    }
}

impl Blueprint for ProjectFile {
    fn render(&self, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "Project.toml")
    }

    fn collect(&self, ctx: &mut Context) -> RenderResult {
        if !ctx.meta.contains_key("version") {
            ctx.meta.insert("version".to_string(), Meta::String(self.version.to_string()));
        }
        ctx.ignore("/Manifest.toml");
        Ok(())
    }
}
