use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("gc")
        .about("garbage collect packages not used for a significant time")
        .arg(arg!(-g --global "Garbage collect the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    "using Pkg; Pkg.gc()".julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
