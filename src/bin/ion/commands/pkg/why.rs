use crate::commands::pkg::package_spec_list;
use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("why")
        .about(
            "Show the reason why packages are in the manifest, printed as a path through \
            the dependency graph starting at the direct dependencies.",
        )
        .arg(arg!([PACKAGE] "The package to inspect").value_hint(ValueHint::AnyPath))
        .arg(arg!(-g --global "Inspect the dependency in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.why([{}])", package_spec_list(matches))
        .julia_exec(&Config::read()?, matches.get_flag("global"))?;
    Ok(())
}
