use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::config::Config;
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("precompile")
        .about("Precompile all packages in the current environment")
        .arg(arg!([PACKAGE] ... "The packages to precompile"))
        .arg(arg!(--strict "Throw errors if any packages fail to precompile"))
        .arg(arg!(-g --global "Precompile the global environment"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let strict = if matches.get_flag("strict") {
        "strict=true"
    } else {
        "strict=false"
    };

    let cmd = if matches.contains_id("PACKAGE") {
        let packages = matches
            .get_many::<String>("PACKAGE")
            .into_iter()
            .flatten()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("using Pkg; Pkg.precompile([{packages}]; {strict})")
    } else {
        format!("using Pkg; Pkg.precompile(;{strict})")
    };

    cmd.julia_exec(&Config::read()?, matches.get_flag("global"))?;
    Ok(())
}
