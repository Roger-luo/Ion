use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::errors::CliResult;
use ion::config::Config;

pub fn cli() -> Command {
    Command::new("release")
        .about("release a new version of a package")
        .arg(arg!(<VERSION> "The version to release"))
        .arg(arg!([PATH] "The path of the package").value_hint(ValueHint::DirPath))
        .arg(arg!(-b --branch [BRANCH] "The branch to release"))
        .arg(arg!(--registry [REGISTRY] "The registry to release").default_value("General"))
        .arg(arg!(--"no-prompt" "Do not prompt for confirmation"))
        .arg(arg!(--"no-commit" "Do not commit changes"))
        .arg(arg!(--"no-report" "Do not report changes"))
        .arg(arg!(--"skip-note" "Skip interactive release note editing"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    crate::commands::bump::exec(config, matches)?;
    crate::commands::summon::exec(config, matches)?;
    Ok(())
}
