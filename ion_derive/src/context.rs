use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::path::PathBuf;

pub fn emit_context() -> TokenStream {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../src/blueprints/components");

    let mut info = quote! {};
    let mut info_defaults = quote! {};
    for entry in d.read_dir().expect("read_dir failed") {
        if let Ok(entry) = entry {
            if entry.path().extension().unwrap().eq("rs") && !entry.file_name().eq("mod.rs") {
                let mod_name = entry.file_name()
                    .into_string()
                    .unwrap()[0..entry.file_name().len() - 3]
                    .to_string();
                let mod_ident = Ident::new(&mod_name, Span::call_site());
                info = quote!{
                    #info
                    pub #mod_ident: Option<super::components::#mod_ident::Info>,
                };
                info_defaults = quote! {
                    #info_defaults
                    #mod_ident: None,
                };
            }
        }
    }

    let gen = quote!{
        use serde_derive::Serialize;

        #[derive(Debug, Serialize, Clone)]
        pub struct Context {
            pub prompt: bool,
            pub julia: Julia,
            pub project: Project,
            pub badges: Vec<Badge>,

            #info
        }

        impl Context {
            pub fn new(prompt: bool, julia: Julia, project: Project) -> Context {
                Context {
                    prompt,
                    julia,
                    project,
                    badges: Vec::new(),
                    #info_defaults
                }
            }
        }
    };
    gen
}
