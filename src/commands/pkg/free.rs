use crate::errors::CliResult;
use clap::{arg, Command};
use clap::parser::ArgMatches;
use crate::julia::Julia;
use crate::commands::pkg::package_spec_list;

pub fn cli() -> Command {
    Command::new("free")
            .about("Free pinned packages in the current environment")
            .arg(arg!([PACKAGE] ... "The package to free"))
            .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!(
        "using Pkg; Pkg.free([{}])",
        package_spec_list(matches)
    ).julia_exec()
}
