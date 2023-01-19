use dialoguer::Input;
use serde_derive::Deserialize;
use crate::blueprints::*;

#[derive(Debug, Deserialize)]
pub struct Readme {
    #[serde(default = "Readme::default_template")]
    template: TemplateFile,
    #[serde(default = "Readme::default_inline_badge")]
    pub inline_badge: bool,
}

impl Readme {
    pub fn default_template() -> TemplateFile {
        TemplateFile::from_str("./README.md.hbs")
    }

    pub fn default_inline_badge() -> bool {
        true
    }
}

impl Blueprint for Readme {
    fn render(&self, ctx: &Context) -> RenderResult {
        self.template.render(ctx, "README.md")
    }

    fn prompt(&self, ctx: &mut Context) -> RenderResult {
        // package name is handled in Context::from
        if !ctx.meta.contains_key("description") {
            let input = Input::<String>::new()
            .with_prompt("description of the project")
            .allow_empty(true)
            .interact_text().expect("error");
            ctx.meta.insert("description".to_string(), Meta::String(input));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Badge {
    hover: String,
    image: String,
    link: String,
}

impl Badge {
    pub fn render(&self) -> String {
        format!(
            "[![{}]({})]({})",
            self.hover, self.image, self.link
        )
    }
}
