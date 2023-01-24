use anyhow::format_err;
use clap::parser::ArgMatches;
use clap::Command;
use ion::errors::CliResult;
use ion::utils::Auth;

pub fn cli() -> Command {
    Command::new("auth")
        .subcommand(Command::new("login").about("login to github"))
        .subcommand(Command::new("logout").about("logout from github"))
        .about("manage authentication")
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let auth = Auth::new(["repo", "read:org"]);
    match matches.subcommand() {
        Some(("login", _)) => {
            auth.get_token()?;
        }
        Some(("logout", _)) => {
            auth.keyring().delete_token()?;
        }
        _ => return Err(format_err!("invalid subcommand").into()),
    }
    Ok(())
}
