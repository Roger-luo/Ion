use dialoguer::{Input, Confirm};
use std::path::PathBuf;
use std::env::current_dir;
use anyhow::Error;
use crate::dirs::template_dir;
use crate::blueprints::{Context, Template, Author, Meta};

pub fn prompt_for_authors(ctx: &mut Context) -> Result<(), Error> {
    if ctx.meta.contains_key("authors") {
        return Ok(());
    }

    let mut authors: Vec<Author> = Vec::new();
    if Confirm::new().with_prompt("author(s) of the project").interact()? {
        authors.push(promot_for_an_author()?);
    } else {
        return Ok(());
    }
    while Confirm::new().with_prompt("another author of the project?").interact()? {
        authors.push(promot_for_an_author()?);
    }

    if Confirm::new().with_prompt("include future contributors as an author?").interact()? {
        authors.push(Author {
            firstname: "other contributors".to_string(),
            lastname: None,
            email: None,
            url: None,
            affiliation: None,
            orcid: None,
        });
    }
    ctx.meta.insert("authors".to_string(), Meta::Authors(authors));
    Ok(())
}

fn promot_for_an_author() -> Result<Author, Error> {
    let firstname = Input::<String>::new()
        .with_prompt("firstname")
        .allow_empty(false)
        .interact_text().expect("error");
    let lastname = promote_for_author_field("lastname");
    let email = promote_for_author_field("email");
    let url = promote_for_author_field("url");
    let affiliation = promote_for_author_field("affiliation");
    let orcid = promote_for_author_field("orcid");
    Ok(Author { firstname, lastname, email, url, affiliation, orcid })
}

fn promote_for_author_field(field: &str) -> Option<String> {
    let input = Input::<String>::new()
        .with_prompt(field)
        .interact_text().expect("error");

    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

pub fn list_templates() {
    template_dir().read_dir().unwrap().for_each(|entry| {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let template: Template = toml::from_str(
                &std::fs::read_to_string(path.join("template.toml")).unwrap()).unwrap();
            println!("
{}
    {}", template.name, template.description);
        }
    });
}

pub fn project_dir(ctx: &Context) -> PathBuf {
    let path = current_dir().unwrap().join(&ctx.name);
    if !path.is_dir() {
        std::fs::create_dir_all(&path).unwrap();
    }
    path
}
