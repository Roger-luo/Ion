use anyhow::format_err;
use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use dialoguer::{Confirm, Input};
use ion::blueprints::*;
use ion::errors::CliResult;
use ion::template::RemoteTemplate;
use log::debug;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("new")
        .about("Create a new package")
        .arg(arg!(path: [PATH] "The path of the package").value_hint(ValueHint::AnyPath))
        .arg(arg!(--list "List available templates"))
        .arg(arg!(-f --force "Overwrite existing files"))
        .arg(arg!(--"no-interactive" "Do not prompt for user input"))
        .arg(
            arg!(template: -t --template <TEMPLATE> "The template to use").default_value("project"),
        )
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    if matches.get_flag("list") {
        list_templates(config)?;
        return Ok(());
    }

    if !config.template_dir().exists() {
        if Confirm::new()
            .with_prompt("Template not found, download from registry?")
            .default(true)
            .interact()?
        {
            RemoteTemplate::new(config).download()?;
        } else {
            return Ok(());
        }
    }

    let prompt = !matches.get_flag("no-interactive");
    let force = matches.get_flag("force");
    let path = match matches.get_one::<String>("path") {
        Some(path) => path.to_owned(),
        None => {
            if prompt {
                Input::<String>::new()
                    .with_prompt("name of the project")
                    .allow_empty(false)
                    .interact_text()
                    .expect("error")
            } else {
                return Err(anyhow::format_err!("No name provided.").into());
            }
        }
    };

    let path = PathBuf::from(path);
    let cwd = std::env::current_dir()?;
    let path = cwd.join(path);

    debug!("path: {}", path.display());
    let package = match path.file_name() {
        Some(name) => name.to_str().unwrap().to_owned(),
        None => return Err(anyhow::format_err!("Invalid path: {}", path.display()).into()),
    };
    mk_package_dir(&path, force)?;

    let mut ctx = Context::new(prompt, Julia::new(&config), Project::new(package, path));
    let name = matches.get_one::<String>("template").unwrap().to_owned();
    let template = Template::from_name(config, &name)?;
    if let Err(e) = template.render(config, &mut ctx) {
        return Err(e.into());
    }
    Ok(())
}

fn mk_package_dir(path: &PathBuf, force: bool) -> CliResult {
    debug!("path: {}", path.display());
    if path.is_dir() {
        if force {
            debug!("removing existing directory: {}", path.display());
            std::fs::remove_dir_all(path)?;
        } else {
            return Err(format_err!("project already exists:{}", path.display()).into());
        }
    }
    std::fs::create_dir_all(path).unwrap();
    Ok(())
}
