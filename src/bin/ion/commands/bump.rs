use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::spec::{JuliaProjectFile, VersionSpec};
use ion::utils::current_project;
use ion::Registry;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("bump")
        .about("bump the version of a package")
        .arg(arg!(<VERSION> "The version to release"))
        .arg(arg!([PATH] "The path of the package").value_hint(ValueHint::DirPath))
        .arg(arg!(-b --branch [BRANCH] "The branch to release"))
        .arg(arg!(--"no-prompt" "Do not prompt for confirmation"))
        .arg(arg!(--"no-commit" "Do not commit changes"))
        .arg(arg!(--"no-report" "Do not report changes"))
        .arg(arg!(--registry [REGISTRY] "The registry to release").default_value("General"))
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
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

    let branch = matches.get_one::<String>("branch");

    log::debug!(
        "bumping version of {} in registry {}",
        path.display(),
        registry_name
    );

    JuliaProjectFile::root_project(path)?
        .bump(&config, version_spec)
        .registry(Registry::read(&config, registry_name)?)?
        .branch(branch)
        .confirm(!matches.get_flag("no-prompt"))
        .report(!matches.get_flag("no-report"))
        .commit(!matches.get_flag("no-commit"))
        .write()?;
    Ok(())
}
