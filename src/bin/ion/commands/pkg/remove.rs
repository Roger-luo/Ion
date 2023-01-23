use clap::{arg, Command};
use clap::parser::ArgMatches;
use ion::errors::CliResult;
use ion::utils::Julia;
use crate::commands::pkg::package_spec_list;


pub fn cli() -> Command {
    Command::new("remove")
        .visible_alias("rm")
        .about("Remove dependencies in the current environment")
        .arg(arg!([PACKAGE] ... "The package to remove"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!(
        "using Pkg; Pkg.rm([{}])",
        package_spec_list(matches)
    ).julia_exec()
}
