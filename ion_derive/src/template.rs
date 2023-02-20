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


    impl fmt::Display for Template {
        fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
            write!(
                format_buffer,
                "Name:\n{}\n\nDescription:\n{}\n\n",
                self.name, self.description
            )?;

            if let Some(repo) = &self.repo {
                write!(format_buffer, "Repo:\n{}\n", repo)?;
            } else {
                write!(format_buffer, "Repo:\nNone\n\n")?;
            }

            if let Some(project_file) = &self.project_file {
                write!(format_buffer, "Project File:\n{}\n", project_file)?;
            } else {
                write!(format_buffer, "Project File:\nNone\n\n")?;
            }

            if let Some(readme) = &self.readme {
                write!(format_buffer, "Readme:\n{}\n", readme)?;
            } else {
                write!(format_buffer, "Readme:\nNone\n\n")?;
            }

            if let Some(src_dir) = &self.src_dir {
                write!(format_buffer, "Source Directory:\n{}\n", src_dir)?;
            } else {
                write!(format_buffer, "Source Directory:\nNone\n\n")?;
            }

            if let Some(tests) = &self.tests {
                write!(format_buffer, "Tests:\n{}\n", tests)?;
            } else {
                write!(format_buffer, "Tests:\nNone\n\n")?;
            }

            if let Some(license_dir) = &self.license {
                write!(format_buffer, "License Template:\n{}\n", license_dir)?;
            } else {
                write!(format_buffer, "License Template:\nNone\n\n")?;
            }

            if let Some(citation) = &self.citation {
                write!(format_buffer, "Citation:\n{}\n", citation)?;
            } else {
                write!(format_buffer, "Citation:\nNone\n\n")?;
            }

            if let Some(documenter) = &self.documenter {
                write!(format_buffer, "Documenter:\n{}\n", documenter)?;
            } else {
                write!(format_buffer, "Documenter:\nNone\n\n")?;
            }

            if let Some(codecov) = &self.codecov {
                write!(format_buffer, "CodeCov:\n{}\n", codecov)?;
            } else {
                write!(format_buffer, "CodeCov:\nNone\n\n")?;
            }

            if let Some(coveralls) = &self.coveralls {
                write!(format_buffer, "Coveralls:\n{}\n", coveralls)?;
            } else {
                write!(format_buffer, "Coveralls:\nNone\n\n")?;
            }
            if let Some(github) = &self.github {
                write!(format_buffer, "Github:\n{}\n", github)?;
            } else {
                write!(format_buffer, "Github:\nNone\n")?;
            }

            Ok(())
        }
    }


        };
    gen.into()
}

// impl fmt::Display for Template {
//     fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             format_buffer,
//             "Name:\n{}\n\nDescription:\n{}\n\n",
//             self.name, self.description
//         )?;

//         if let Some(repo) = &self.repo {
//             write!(format_buffer, "Repo:\n{repo}\n")?;
//         } else {
//             write!(format_buffer, "Repo:\nNone\n\n")?;
//         }

//         if let Some(project_file) = &self.project_file {
//             write!(format_buffer, "Project File:\n{project_file}\n")?;
//         } else {
//             write!(format_buffer, "Project File:\nNone\n\n")?;
//         }

//         if let Some(readme) = &self.readme {
//             write!(format_buffer, "Readme:\n{readme}\n")?;
//         } else {
//             write!(format_buffer, "Readme:\nNone\n\n")?;
//         }

//         if let Some(src_dir) = &self.src_dir {
//             write!(format_buffer, "Source Directory:\n{src_dir}\n")?;
//         } else {
//             write!(format_buffer, "Source Directory:\nNone\n\n")?;
//         }

//         if let Some(tests) = &self.tests {
//             write!(format_buffer, "Tests:\n{tests}\n")?;
//         } else {
//             write!(format_buffer, "Tests:\nNone\n\n")?;
//         }

//         if let Some(license_dir) = &self.license {
//             write!(format_buffer, "License Template:\n{license_dir}\n")?;
//         } else {
//             write!(format_buffer, "License Template:\nNone\n\n")?;
//         }

//         if let Some(citation) = &self.citation {
//             write!(format_buffer, "Citation:\n{citation}\n")?;
//         } else {
//             write!(format_buffer, "Citation:\nNone\n\n")?;
//         }

//         if let Some(documenter) = &self.documenter {
//             write!(format_buffer, "Documenter:\n{documenter}\n")?;
//         } else {
//             write!(format_buffer, "Documenter:\nNone\n\n")?;
//         }

//         if let Some(codecov) = &self.codecov {
//             write!(format_buffer, "CodeCov:\n{codecov}\n")?;
//         } else {
//             write!(format_buffer, "CodeCov:\nNone\n\n")?;
//         }

//         if let Some(coveralls) = &self.coveralls {
//             write!(format_buffer, "Coveralls:\n{coveralls}\n")?;
//         } else {
//             write!(format_buffer, "Coveralls:\nNone\n\n")?;
//         }
//         if let Some(github) = &self.github {
//             write!(format_buffer, "Github:\n{github}\n")?;
//         } else {
//             write!(format_buffer, "Github:\nNone\n")?;
//         }

//         Ok(())
//     }
// }
