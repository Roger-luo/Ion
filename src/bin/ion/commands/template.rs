use clap::parser::ArgMatches;
use clap::Command;
use ion::blueprints::list_templates;
use ion::errors::CliResult;
use ion::template::RemoteTemplate;
use ion::config::Config;

pub fn cli() -> Command {
    Command::new("template")
        .about("template management")
        .subcommand(Command::new("list").about("list all available templates"))
        .subcommand(Command::new("update").about("update the templates from registry"))
}

pub fn exec(_config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("list", _)) => list_templates()?,
        Some(("update", _)) => {
            RemoteTemplate::default().download()?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
