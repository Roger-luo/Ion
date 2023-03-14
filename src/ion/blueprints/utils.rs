use crate::blueprints::*;
use crate::spec::Author;
use anyhow::{format_err, Error, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

pub fn git_get_user() -> Result<(String, String)> {
    let user = if let Some(name) = git_config_get("user.name") {
        name
    } else {
        return Err(format_err!("user.name not set in git config"));
    };

    let email = if let Some(email) = git_config_get("user.email") {
        email
    } else {
        return Err(format_err!("user.email not set in git config"));
    };
    Ok((user, email))
}

pub fn prompt_for_authors() -> Result<Vec<Author>> {
    let mut authors = Vec::<Author>::new();
    authors.push(promot_for_an_author()?);
    while Confirm::new()
        .with_prompt("another author of the project?")
        .default(false)
        .interact()?
    {
        authors.push(promot_for_an_author()?);
    }

    if Confirm::new()
        .with_prompt("include future contributors as an author?")
        .default(true)
        .interact()?
    {
        authors.push(Author {
            firstname: "and contributors".to_string(),
            lastname: None,
            email: None,
            url: None,
            affiliation: None,
            orcid: None,
        });
    }
    Ok(authors)
}

fn promot_for_an_author() -> Result<Author> {
    let firstname = Input::<String>::new()
        .with_prompt("firstname")
        .allow_empty(false)
        .interact_text()
        .expect("error");
    let lastname = promote_for_author_field("lastname");
    let email = promote_for_author_field("email");
    let url = promote_for_author_field("url");
    let affiliation = promote_for_author_field("affiliation");
    let orcid = promote_for_author_field("orcid");
    Ok(Author {
        firstname,
        lastname,
        email,
        url,
        affiliation,
        orcid,
    })
}

fn promote_for_author_field(field: &str) -> Option<String> {
    let input = Input::<String>::new()
        .with_prompt(format!("{field} (optional)"))
        .allow_empty(true)
        .interact_text()
        .expect("error");

    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

pub fn list_templates(config: &Config) -> Result<()> {
    let templates = config.template_dir().read_dir()?;

    for entry in templates {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            let source = std::fs::read_to_string(path.join("template.toml"))?;
            let template = match toml::from_str::<Template>(&source) {
                Ok(t) => t,
                Err(e) => {
                    return Err(format_err!("Error parsing template: {}", e));
                }
            };
            println!(
                "
{}
    {}",
                template.name, template.description
            );
        }
    }
    Ok(())
}

pub fn inspect_template(config: &Config, template_name: String, verbose_flag: bool) -> Result<()> {
    let templates = config.template_dir().read_dir()?;

    let mut template_found: bool = false;

    for entry in templates {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            let source = std::fs::read_to_string(path.join("template.toml"))?;

            let template = match toml::from_str::<Template>(&source) {
                Ok(t) => t,
                Err(e) => {
                    return Err(format_err!("Error parsing template: {}", e));
                }
            };
            if template.name == template_name {
                // If there's no verbose flag (default), print the source, otherwise, display the full template details (verbose true)
                if verbose_flag {
                    template_found = true;
                    println!("{template:#?}");
                } else {
                    template_found = true;
                    println!("{source}");
                }
            }
        }
    }

    // If the template the user requested is not in the list of downloaded templates, ask user to select existing template to inspect
    if !template_found {
        println!("The {template_name} template was not found.\nInstalled templates are:");

        ask_inspect_template(config, verbose_flag)?
    }
    Ok(())
}

pub fn ask_inspect_template(config: &Config, verbose_flag: bool) -> Result<()> {
    // Get selection options from installed templates
    let mut selection_options = vec![];

    let templates = config.template_dir().read_dir()?;

    for entry in templates {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            let source = std::fs::read_to_string(path.join("template.toml"))?;

            let template = match toml::from_str::<Template>(&source) {
                Ok(t) => t,
                Err(e) => {
                    return Err(format_err!("Error parsing template: {}", e));
                }
            };

            selection_options.push(template.name);
        }
    }

    // Ask for template to print to console
    let template_name = Select::with_theme(&ColorfulTheme::default())
        .items(&selection_options)
        .default(1)
        .interact_opt()?;

    if let Some(template_name) = template_name {
        inspect_template(
            config,
            selection_options[template_name].to_owned(),
            verbose_flag,
        )?
    };

    Ok(())
}

pub fn inspect_all_templates(config: &Config, verbose_flag: bool) -> Result<()> {
    let templates = config.template_dir().read_dir()?;

    for entry in templates {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            let source = std::fs::read_to_string(path.join("template.toml"))?;

            let template = match toml::from_str::<Template>(&source) {
                Ok(t) => t,
                Err(e) => {
                    return Err(format_err!("Error parsing template: {}", e));
                }
            };

            if verbose_flag {
                println!("\n{template:#?}\n**********");
            } else {
                println!("\n{source}\n**********");
            }
        }
    }

    Ok(())
}

pub fn git_config_get(key: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("config")
        .arg("--get")
        .arg(key)
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            return Some(String::from_utf8(o.stdout).unwrap().trim().to_string());
        }
    }
    None
}

pub fn git_current_branch() -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            return Some(String::from_utf8(o.stdout).unwrap().trim().to_string());
        }
    }
    None
}

pub fn git_checkout(branch: &String) -> Result<()> {
    std::process::Command::new("git")
        .arg("checkout")
        .arg("-b")
        .arg(branch)
        .status()?;
    Ok(())
}

pub fn git_delete_branch(branch: &String) -> Result<()> {
    std::process::Command::new("git")
        .arg("branch")
        .arg("-D")
        .arg(branch)
        .status()?;
    Ok(())
}
