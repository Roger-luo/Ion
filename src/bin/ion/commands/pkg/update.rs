use crate::commands::*;

pub fn cli() -> Command {
    Command::new("update")
        .visible_alias("up")
        .about("Update the current environment")
        .arg(arg!([PACKAGE] ... "The package to update"))
        .arg(arg!(-g --global "Update the global environment"))
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.update({})", PackageSpecList::new(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
