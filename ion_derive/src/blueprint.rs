use crate::utils::emit_field_calls;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, DeriveInput};

pub fn emit_blueprint(ast: &DeriveInput) -> TokenStream {
    let t = &Ident::new("t", Span::call_site());
    let name = &ast.ident;
    let render = emit_field_calls(&ast, t, "render");
    let collect = emit_field_calls(&ast, t, "collect");
    let prompt = emit_field_calls(&ast, t, "prompt");
    let post_render = emit_field_calls(&ast, t, "post_render");
    let validate = emit_field_calls(&ast, t, "validate");

    let gen = quote! {
        use super::Blueprint;
        impl Blueprint for #name {
            pub fn render(&self, t: &Template, ctx: &mut Context) -> Result<(), anyhow::Error> {
                #render
                Ok(())
            }

            pub fn collect(&self, t: &Template, ctx: &mut Context) -> Result<(), anyhow::Error> {
                #collect
                Ok(())
            }

            pub fn prompt(&self, t: &Template, ctx: &mut Context) -> Result<(), anyhow::Error> {
                #prompt
                Ok(())
            }

            pub fn post_render(&self, t: &Template, ctx: &Context) -> Result<(), anyhow::Error> {
                #post_render
                Ok(())
            }

            pub fn validate(&self, t: &Template, ctx: &Context) -> Result<(), anyhow::Error> {
                #validate
                Ok(())
            }
        }
    };
    gen.into()
}
