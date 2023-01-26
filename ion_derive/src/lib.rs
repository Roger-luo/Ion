use proc_macro::TokenStream;
use syn::{self, DeriveInput};

mod blueprint;
mod context;
mod template;
mod utils;

#[proc_macro_derive(Template)]
pub fn template_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    template::emit_template(&ast)
}

#[proc_macro_derive(Blueprint)]
pub fn blueprint_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();
    blueprint::emit_blueprint(&ast)
}
