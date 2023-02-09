use crate::commands::pkg::package_spec_list;
use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("add")
        .about("Add dependencies to current environment")
        .arg(arg!([PACKAGE] ... "The package to add").value_hint(ValueHint::AnyPath))
        .arg(arg!(-g --global "Add the package to the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.add([{}])", package_spec_list(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
