use crate::commands::pkg::package_spec_list;
use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("update")
        .visible_alias("up")
        .about("Update the current environment")
        .arg(arg!([PACKAGE] ... "The package to update"))
        .arg(arg!(-g --global "Update the global environment"))
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    let cmd = if matches.args_present() {
        format!("using Pkg; Pkg.update([{}])", package_spec_list(matches))
    } else {
        "using Pkg; Pkg.update()".into()
    };
    cmd.julia_exec(config, matches.get_flag("global"))?;

    Ok(())
}
