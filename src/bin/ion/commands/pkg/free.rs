use crate::commands::*;

pub fn cli() -> Command {
    Command::new("free")
        .about("Free pinned packages in the current environment")
        .arg(arg!([PACKAGE] ... "The package to free"))
        .arg(arg!(-g --global "Free the package in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.free({})", PackageSpecList::new(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
