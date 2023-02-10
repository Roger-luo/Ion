use crate::commands::*;

pub fn cli() -> Command {
    Command::new("remove")
        .visible_alias("rm")
        .about("Remove dependencies in the current environment")
        .arg(arg!([PACKAGE] ... "The package to remove"))
        .arg(arg!(-g --global "Remove the package in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.rm({})", PackageSpecList::new(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
