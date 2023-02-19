use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::blueprints::{
    ask_inspect_template, inspect_all_templates, inspect_template, list_templates,
};
use ion::config::Config;
use ion::errors::CliResult;
use ion::template::RemoteTemplate;

pub fn cli() -> Command {
    Command::new("template")
        .about("template management")
        .subcommand(Command::new("list").about("list all available templates"))
        .subcommand(Command::new("update").about("update the templates from registry"))
        .subcommand(
            Command::new("inspect")
                .about("inspect the contents of a template")
                .arg(arg!([TEMPLATE] "Selects which template to print out"))
                .arg(arg!(--"all" "Inspect all installed templates")),
        )
        .arg_required_else_help(true)
}

fn download_templates(config: &mut Config) -> CliResult {
    if !config.resources().exists() && dialoguer::Confirm::new()
        .with_prompt("No templates found. Would you like to download them now?")
        .default(true)
        .interact()? {
        RemoteTemplate::new(config).download()?;
    }
    Ok(())
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("list", _)) => {
            download_templates(config)?;
            list_templates(config)?
        },
        Some(("update", _)) => {
            RemoteTemplate::new(config).download()?;
        }
        Some(("inspect", matches)) => {
            download_templates(config)?;
            // Iff a template name is provided, inspect template; otherwise, check for --all flag; if no --all, ask user to select template from list

            match matches.get_one::<String>("TEMPLATE") {
                Some(template) => inspect_template(config, template.to_owned())?,
                None => {
                    let all_flag = matches.get_flag("all");
                    if all_flag {
                        inspect_all_templates(config)?;
                    } else {
                        ask_inspect_template(config)?;
                    }
                }
            };
        }
        _ => unreachable!(),
    }
    Ok(())
}
