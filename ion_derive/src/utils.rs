use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{self, DeriveInput};

pub fn emit_field_calls(
    ast: &DeriveInput,
    template: &Ident,
    method_name: &str,
) -> proc_macro2::TokenStream {
    // Build the trait implementation
    let data = &ast.data;
    let mut gen = quote! {};
    let func = format_ident!("{}", method_name);

    if let syn::Data::Struct(data) = data {
        let fields = &data.fields;
        if let syn::Fields::Named(fields) = fields {
            let fields = &fields.named;
            for field in fields {
                let field_name = field.ident.as_ref().unwrap();
                gen = quote! {
                    #gen
                    self.#field_name.#func(#template, config, ctx)?;
                };
            }
        }
    }
    gen
}
