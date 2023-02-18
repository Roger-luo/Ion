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
                .arg(arg!(--"all" "Inspect all installed templates"))
                .arg(arg!(verbose: -v --verbose "Inspect details of the template output")),

        )
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("list", _)) => list_templates(config)?,
        Some(("update", _)) => {
            RemoteTemplate::new(config).download()?;
        }
        Some(("inspect", matches)) => {
            let all_flag = matches.get_flag("all");
            let verbose_flag = matches.get_flag("verbose");

            // Iff a template name is provided, inspect template; otherwise, check for --all flag; if no --all, ask user to select template from list
            match matches.get_one::<String>("TEMPLATE") {
                Some(template) => {
                    inspect_template(config, template.to_owned(), verbose_flag)?;
                }
                None => {
                    if all_flag {
                        inspect_all_templates(config, verbose_flag)?;
                    } else {
                        ask_inspect_template(config, verbose_flag)?;

                    }
                }
            };
        }
        _ => unreachable!(),
    }
    Ok(())
}
