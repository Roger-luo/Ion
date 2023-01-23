use dialoguer::{Input, Confirm};
use std::process::Command;
use anyhow::{Error, format_err};
use crate::utils::template_dir;
use crate::blueprints::*;

pub fn git_get_user() -> Result<(String, String), Error> {
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

pub fn prompt_for_authors() -> Result<Vec<Author>, Error> {
    let mut authors = Vec::<Author>::new();
    authors.push(promot_for_an_author()?);
    while Confirm::new()
        .with_prompt("another author of the project?")
        .default(false)
        .interact()? {
        authors.push(promot_for_an_author()?);
    }

    if Confirm::new()
        .with_prompt("include future contributors as an author?")
        .default(true)
        .interact()? {
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
        .with_prompt(format!("{} (optional)", field))
        .allow_empty(true)
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

pub fn julia_version() -> Result<String, Error> {
    let output = Command::new("julia")
        .arg("--version")
        .output();
    
    match output {
        Err(e) => return Err(Error::new(e)),
        Ok(output) => {
            let version = String::from_utf8(output.stdout)?;
            let version = version.trim();
            return Ok(version.to_string());
        }
    }
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
    return None;
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
    return None;
}

pub fn git_checkout(branch: &String) -> Result<(), Error> {
    std::process::Command::new("git")
        .arg("checkout").arg("-b").arg(branch)
        .status()?;
    Ok(())
}

pub fn git_delete_branch(branch: &String) -> Result<(), Error> {
    std::process::Command::new("git")
        .arg("branch").arg("-D").arg(branch)
        .status()?;
    Ok(())
}
