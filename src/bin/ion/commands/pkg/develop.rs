use crate::commands::*;

pub fn cli() -> Command {
    Command::new("develop")
        .visible_alias("dev")
        .about("Develop packages in the current environment")
        .arg(arg!([PACKAGE] "The package path to develop").value_hint(ValueHint::AnyPath))
        .arg(arg!(-v --verbose "show detailed output"))
        .arg(arg!(--all "garbage collect all packages which can not \
            be immediately reached from existing project"))
        .arg(arg!(-g --global "develop the package in the global environment"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    format!("using Pkg; Pkg.develop({})", PackageSpecList::new(matches))
        .julia_exec(config, matches.get_flag("global"))?;
    Ok(())
}
