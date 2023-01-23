use ion::errors::CliResult;
use clap::{arg, Command};
use clap::parser::ArgMatches;
use ion::utils::Julia;
use crate::commands::pkg::package_spec_list;

pub fn cli() -> Command {
    Command::new("develop")
        .visible_alias("dev")
        .about("Develop packages in the current environment")
        .arg(arg!([PACKAGE] "The package path to develop"))
        .arg(arg!(-v --verbose "show detailed output"))
        .arg(arg!(--all "garbage collect all packages which can not \
            be immediately reached from existing project"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!(
        "using Pkg; Pkg.develop([{}])",
        package_spec_list(matches)
    ).julia_exec()
}
