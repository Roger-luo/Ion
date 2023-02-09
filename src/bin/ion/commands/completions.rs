use anyhow::format_err;
use clap::{arg, value_parser, ArgMatches, Command};
use clap_complete::{generate, Shell};
use ion::config::Config;
use ion::errors::CliResult;
use std::io;

pub fn cli() -> Command {
    Command::new("completions")
        .about("generate shell completion scripts")
        .arg(
            arg!([SHELL] "The shell to generate completions for")
                .value_parser(value_parser!(Shell)),
        )
}

pub fn exec(_config: &mut Config, matches: &ArgMatches) -> CliResult {
    if let Some(shell) = matches.get_one::<Shell>("SHELL").copied() {
        let mut cmd = crate::cli();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut io::stdout());
    } else {
        return Err(format_err!("No shell provided").into());
    }
    Ok(())
}
