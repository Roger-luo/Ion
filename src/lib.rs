use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, format_ident};
use syn::{self, DeriveInput};

#[proc_macro_derive(Template)]
pub fn template_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let t = &Ident::new("self", Span::call_site());
    let render = emit_field_calls(&ast, t,  "render");
    let collect = emit_field_calls(&ast, t,  "collect");
    let prompt = emit_field_calls(&ast, t,  "prompt");
    let post_render = emit_field_calls(&ast, t,  "post_render");
    let validate = emit_field_calls(&ast, t,  "validate");

    let gen = quote!{
        impl #name {
            pub fn from_name(name: &String) -> #name {
                let mut template = template_dir();
                template.push(name);
                template.push("template.toml");

                assert!(template.is_file(), "Template file not found: {}", template.display());
                let template : Template = toml::from_str(
                    &std::fs::read_to_string(template).unwrap()).unwrap();
                template
            }

            pub fn render(&self, ctx: &mut Context) -> RenderResult {
                let old_pwd = std::env::current_dir()?;
                std::env::set_current_dir(&*ctx.project.path)?;

                self.collect(ctx)?;
                debug!("Context: {:#?}", ctx);
                if ctx.prompt {
                    self.prompt(ctx)?;
                }
                #render
                self.post_render(ctx)?;
                self.validate(ctx)?;

                std::env::set_current_dir(old_pwd)?;
                Ok(())
            }

            pub fn collect(&self, ctx: &mut Context) -> RenderResult {
                #collect
                Ok(())
            }

            pub fn prompt(&self, ctx: &mut Context) -> RenderResult {
                #prompt
                Ok(())
            }

            pub fn post_render(&self, ctx: &Context) -> RenderResult {
                #post_render
                Ok(())
            }

            pub fn validate(&self, ctx: &Context) -> RenderResult {
                #validate
                Ok(())
            }
        }
    };
    gen.into()
}


#[proc_macro_derive(Blueprint)]
pub fn blueprint_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let t = &Ident::new("t", Span::call_site());
    let name = &ast.ident;
    let render = emit_field_calls(&ast, t, "render");
    let collect = emit_field_calls(&ast, t,  "collect");
    let prompt = emit_field_calls(&ast, t,  "prompt");
    let post_render = emit_field_calls(&ast, t,  "post_render");
    let validate = emit_field_calls(&ast, t, "validate");

    let gen = quote!{
        impl Blueprint for #name {
            pub fn render(&self, t: &Template, ctx: &mut Context) -> RenderResult {
                #render
                Ok(())
            }

            pub fn collect(&self, t: &Template, ctx: &mut Context) -> RenderResult {
                #collect
                Ok(())
            }

            pub fn prompt(&self, t: &Template, ctx: &mut Context) -> RenderResult {
                #prompt
                Ok(())
            }

            pub fn post_render(&self, t: &Template, ctx: &Context) -> RenderResult {
                #post_render
                Ok(())
            }

            pub fn validate(&self, t: &Template, ctx: &Context) -> RenderResult {
                #validate
                Ok(())
            }
        }
    };
    gen.into()
}

fn emit_field_calls(ast: &DeriveInput, template: &Ident,  method_name: &str) -> proc_macro2::TokenStream {
// Build the trait implementation
    let data = &ast.data;
    let mut gen = quote!{};
    let func = format_ident!("{}", method_name);

    if let syn::Data::Struct(data) = data {
        let fields = &data.fields;
        if let syn::Fields::Named(fields) = fields {
            let fields = &fields.named;
            for field in fields {
                let field_name = field.ident.as_ref().unwrap();
                gen = quote! {
                    #gen
                    self.#field_name.#func(#template, ctx)?;
                };
            }
        }
    }
    gen.into()
}
