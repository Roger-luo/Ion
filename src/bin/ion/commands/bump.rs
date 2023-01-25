use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::errors::CliResult;
use ion::spec::{VersionSpec, JuliaProjectFile};
use ion::utils::current_project;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("bump")
        .about("bump the version of a package")
        .arg(arg!(<VERSION> "The version to release"))
        .arg(arg!([PATH] "The path of the package"))
        .arg(arg!(--no-prompt "Do not prompt for confirmation"))
        .arg(arg!(--no-commit "Do not commit changes"))
        .arg(arg!(--registry [REGISTRY] "The registry to release").default_value("General"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let version_spec = match matches.get_one::<String>("VERSION") {
        Some(version) => VersionSpec::from_string(version)?,
        None => return Err(anyhow::format_err!("No version provided.").into()),
    };

    let path = match matches.get_one::<String>("PATH") {
        Some(path) => PathBuf::from(path),
        None => match current_project(std::env::current_dir()?) {
            Some(path) => path,
            None => return Err(anyhow::format_err!("cannot find valid Project.toml").into()),
        },
    };

    let registry_name = match matches.get_one::<String>("registry") {
        Some(registry) => registry.to_owned(),
        None => "General".to_owned(),
    };

    JuliaProjectFile::root_project(path)?
        .bump(version_spec)?
        .registry(registry_name)
        .confirm(matches.get_flag("no-prompt"))?
        .write()?
        .commit(matches.get_flag("no-commit"))?;
    Ok(())
}
