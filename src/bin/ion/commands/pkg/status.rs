use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("status")
        .visible_alias("st")
        .about("Show the status of the current environment")
        .arg(arg!(--outdated "only show packages that are not on the latest version"))
        .arg(arg!(--"no-diff" "do not show diff of packages that are not on the latest version"))
        .arg(arg!(--manifest "show the status of the manifest file"))
        .arg(arg!(-g --global "show the status of the global environment"))
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    let mut options = Vec::<String>::new();
    if matches.get_flag("outdated") {
        options.push("outdated=true".to_string());
    }
    if matches.get_flag("no-diff") {
        options.push("diff=false".to_string());
    }
    if matches.get_flag("manifest") {
        options.push("manifest=true".to_string());
    }
    format!("using Pkg; Pkg.status(;{})", options.join(", "))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
