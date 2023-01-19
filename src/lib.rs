use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{self, DeriveInput};

#[proc_macro_derive(Blueprint)]
pub fn blueprint_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // Build the trait implementation
    let name = &ast.ident;
    let render = emit_field_calls(&ast, "render");
    let collect = emit_field_calls(&ast, "collect");
    let prompt = emit_field_calls(&ast, "prompt");
    let post_render = emit_field_calls(&ast, "post_render");

    let gen = quote!{
        impl Blueprint for #name {
            fn render(&self, ctx: & Context) -> RenderResult {
                #render
                Ok(())
            }

            fn collect(&self, ctx: &mut Context) -> RenderResult {
                #collect
                Ok(())
            }

            fn prompt(&self, ctx: &mut Context) -> RenderResult {
                #prompt
                Ok(())
            }

            fn post_render(&self, ctx: &Context) -> RenderResult {
                #post_render
                Ok(())
            }
        }
    };
    gen.into()
}

fn emit_field_calls(ast: &DeriveInput, method_name: &str) -> proc_macro2::TokenStream {
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
                    self.#field_name.#func(ctx)?;
                };
            }
        }
    }
    gen.into()
}
