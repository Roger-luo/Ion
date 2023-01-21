use clap::{arg, Command};
use clap::parser::ArgMatches;
use log::debug;
use dialoguer::Input;
use std::path::PathBuf;
use anyhow::format_err;
use crate::errors::{CliError, CliResult};
use crate::blueprints::*;

pub fn cli() -> Command {
    Command::new("new")
        .about("Create a new package")
        .arg(arg!(name: [NAME] "The name of the package"))
        .arg(arg!(--list "List available templates"))
        .arg(arg!(-f --force "Overwrite existing files"))
        .arg(arg!(--"no-interactive" "Do not prompt for user input"))
        .arg(
            arg!(template: -t --template <TEMPLATE> "The template to use")
                .default_value("project")
        ) 
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    if matches.get_flag("list") {
        list_templates();
        return Ok(());
    }
    let prompt = !matches.get_flag("no-interactive");
    let force = matches.get_flag("force");
    let package = match matches.get_one::<String>("name") {
        Some(name) => name.to_owned(),
        None => {
            if prompt {
                Input::<String>::new()
                    .with_prompt("name of the project")
                    .allow_empty(false)
                    .interact_text().expect("error")
            } else {
                return Err(anyhow::format_err!("No name provided.").into())
            }
        },
    };
    let path = std::env::current_dir().unwrap().join(package.to_owned());
    mk_package_dir(&path, force)?;

    let mut ctx = Context::new(prompt, Julia::default(), Project::new(package, path));
    let name = matches.get_one::<String>("template").unwrap().to_owned();

    let template = Template::from_name(&name);
    if let Err(e) = template.render(&mut ctx) {
        return Err(CliError::new(e, 1));
    }
    Ok(())
}

fn mk_package_dir(path: &PathBuf, force: bool) -> CliResult {
    debug!("path: {}", path.display());
    if path.is_dir() {
        if force {
            debug!("removing existing directory: {}", path.display());
            std::fs::remove_dir_all(&path)?;
        } else {
            return Err(format_err!("project already exists:{}", path.display()).into())
        }
    }
    std::fs::create_dir_all(&path).unwrap();
    Ok(())
}
