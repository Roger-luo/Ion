use clap::{Command, arg};
use clap::parser::ArgMatches;
use ion::errors::CliResult;
use ion::julia::Julia;
use crate::commands::pkg::package_spec_list;

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
        ).julia_exec()
    } else {
        "using Pkg; Pkg.update()".julia_exec()
    }
}
