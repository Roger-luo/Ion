use crate::commands::pkg::package_spec_list;
use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::{assert_julia_version, Julia};

pub fn cli() -> Command {
    Command::new("why")
        .about("show why a package is installed")
        .long_about(
            "Show the reason why packages are in the manifest, printed as a path through \
            the dependency graph starting at the direct dependencies.",
        )
        .arg(arg!([PACKAGE] "The package to inspect").value_hint(ValueHint::AnyPath))
        .arg(arg!(-g --global "Inspect the dependency in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    assert_julia_version(config, ">=1.9.0-beta")?;
    format!("using Pkg; Pkg.why([{}])", package_spec_list(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
