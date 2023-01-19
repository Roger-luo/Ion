use cargo::CliResult;
use clap::{arg, Command};
use clap::parser::ArgMatches;
use crate::commands::pkg::{JuliaCmd, package_spec_list};

pub fn cli() -> Command {
    Command::new("add")
        .about("Add dependencies to current environment")
        .arg(arg!([PACKAGE] ... "The package to add"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!(
        "using Pkg; Pkg.add([{}])",
        package_spec_list(matches)
    ).as_julia_script()
}
