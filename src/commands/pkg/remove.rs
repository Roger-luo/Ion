use clap::{arg, Command};
use clap::parser::ArgMatches;
use cargo::CliResult;
use crate::commands::pkg::{JuliaCmd, package_spec_list};


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
    ).as_julia_script()
}
