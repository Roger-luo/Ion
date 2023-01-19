use clap::{arg, Command};
use clap::parser::ArgMatches;
use cargo::{CliResult, CliError};
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

    let name = matches.get_one::<String>("template").unwrap().to_owned();
    let template = Template::load(&name);

    let ctx = match Context::from(&template, &matches) {
        Ok(ctx) => ctx,
        Err(e) => return Err(CliError::new(e, 1)),
    };

    if let Err(e) = template.render(&ctx) {
        return Err(CliError::new(e, 1));
    }
    match template.post_render(&ctx) {
        Ok(_) => Ok(()),
        Err(e) => return Err(CliError::new(e, 1)),
    }
}
