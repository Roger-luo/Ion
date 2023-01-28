use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::errors::CliResult;
use ion::utils::Julia;

pub fn cli() -> Command {
    Command::new("precompile")
        .about("Precompile all packages in the current environment")
        .arg(arg!([PACKAGE] ... "The packages to precompile"))
        .arg(arg!(--strict "Throw errors if any packages fail to precompile"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let strict = if matches.get_flag("strict") {
        "strict=true"
    } else {
        "strict=false"
    };

    if matches.contains_id("PACKAGE") {
        let packages = matches
            .get_many::<String>("PACKAGE")
            .into_iter()
            .flatten()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("using Pkg; Pkg.precompile([{packages}]; {strict})").julia_exec()?;
    } else {
        format!("using Pkg; Pkg.precompile(;{strict})").julia_exec()?;
    }
    Ok(())
}
