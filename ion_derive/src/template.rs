use crate::context::emit_context;
use crate::utils::emit_field_calls;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, DeriveInput};

pub fn emit_template(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let t = &Ident::new("self", Span::call_site());
    let render = emit_field_calls(ast, t, "render");
    let collect = emit_field_calls(ast, t, "collect");
    let prompt = emit_field_calls(ast, t, "prompt");
    let post_render = emit_field_calls(ast, t, "post_render");
    let validate = emit_field_calls(ast, t, "validate");

    let context_expr = emit_context();

    let gen = quote! {
        use log::debug;
        use anyhow::Result;
        use crate::config::Config;

        #context_expr

        impl #name {
            pub fn from_name(config: &Config, name: &String) -> Result<Self> {
                let mut template = config.template_dir();
                template.push(name);
                template.push("template.toml");

                assert!(template.is_file(), "Template file not found: {}", template.display());
                let source = std::fs::read_to_string(template)?;
                let template : Template = toml::from_str(&source)?;
                Ok(template)
            }

            pub fn render(&self, config: &Config, ctx: &mut Context) -> Result<()> {
                let old_pwd = std::env::current_dir()?;
                std::env::set_current_dir(&*ctx.project.path)?;

                self.collect(config, ctx)?;
                debug!("Context: {:#?}", ctx);
                if ctx.prompt {
                    self.prompt(config, ctx)?;
                }
                #render
                self.post_render(config, ctx)?;
                self.validate(config, ctx)?;

                std::env::set_current_dir(old_pwd)?;
                Ok(())
            }

            pub fn collect(&self, config: &Config, ctx: &mut Context) -> Result<()> {
                #collect
                Ok(())
            }

            pub fn prompt(&self, config: &Config, ctx: &mut Context) -> Result<()> {
                #prompt
                Ok(())
            }

            pub fn post_render(&self, config: &Config, ctx: &Context) -> Result<()> {
                #post_render
                Ok(())
            }

            pub fn validate(&self, config: &Config, ctx: &Context) -> Result<()> {
                #validate
                Ok(())
            }
        }
    };
    gen.into()
}
