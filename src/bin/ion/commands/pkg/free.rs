use crate::commands::pkg::package_spec_list;
use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("free")
        .about("Free pinned packages in the current environment")
        .arg(arg!([PACKAGE] ... "The package to free"))
        .arg(arg!(-g --global "Free the package in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.free([{}])", package_spec_list(matches))
        .julia_exec(matches.get_flag("global"))?;
    Ok(())
}
