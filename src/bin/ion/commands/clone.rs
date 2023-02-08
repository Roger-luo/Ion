use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::clone;
use ion::errors::CliResult;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("clone")
        .about("Clone a package from URL or registry")
        .arg(arg!(url_or_name: <URL> "The name/url of the package").value_hint(ValueHint::Url))
        .arg(arg!(dest: [PATH] "The path of the package").value_hint(ValueHint::AnyPath))
        .arg(arg!(registry: --registry [REGISTRY] "The registry to use").default_value("General"))
        .arg(arg!(force: -f --force "Force clone to destination"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let url_or_name = matches.get_one::<String>("url_or_name").unwrap().to_owned();
    let dest = matches.get_one::<PathBuf>("dest");
    let registry_name = matches.get_one::<String>("registry").unwrap().to_owned();

    clone::Clone::new(registry_name)
        .from_github(url_or_name)?
        .dest(dest)?
        .run(matches.get_flag("force"))?;
    Ok(())
}
