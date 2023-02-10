use clap::parser::ArgMatches;
use clap::Command;
use ion::blueprints::list_templates;
use ion::config::Config;
use ion::errors::CliResult;
use ion::template::RemoteTemplate;

pub fn cli() -> Command {
    Command::new("template")
        .about("template management")
        .subcommand(Command::new("list").about("list all available templates"))
        .subcommand(Command::new("update").about("update the templates from registry"))
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("list", _)) => list_templates(config)?,
        Some(("update", _)) => {
            RemoteTemplate::new(&config).download()?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
