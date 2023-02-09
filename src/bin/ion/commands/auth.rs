use anyhow::format_err;
use clap::parser::ArgMatches;
use clap::Command;
use ion::config::Config;
use ion::errors::CliResult;

pub fn cli() -> Command {
    Command::new("auth")
        .subcommand(Command::new("login").about("login to github"))
        .subcommand(Command::new("logout").about("logout from github"))
        .about("manage authentication")
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("login", _)) => {
            config.login()?;
        }
        Some(("logout", _)) => {
            config.logout()?;
        }
        _ => return Err(format_err!("invalid subcommand").into()),
    }
    Ok(())
}
