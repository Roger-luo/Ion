use clap::{Command, arg};
use clap::parser::ArgMatches;
use cargo::CliResult;
use crate::commands::pkg::{JuliaCmd, package_spec_list};

pub fn cli() -> Command {
    Command::new("update")
        .visible_alias("up")
        .about("Update the current environment")
        .arg(arg!([PACKAGE] ... "The package to update"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    if matches.args_present() {
        format!(
            "using Pkg; Pkg.update([{}])",
            package_spec_list(matches)
        ).as_julia_script()
    } else {
        "using Pkg; Pkg.update()".as_julia_script()
    }
}
