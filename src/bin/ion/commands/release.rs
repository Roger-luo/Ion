use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::errors::CliResult;
use ion::release::handler::Release;
use ion::release::version_spec::VersionSpec;
use ion::utils::current_project;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("release")
        .about("release a new version of a package")
        .arg(arg!(<VERSION> "The version to release"))
        .arg(arg!([PATH] "The path of the package"))
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

    let mut info = Release::plan(path, version_spec, registry_name)?;
    let mut handler = info.ask_branch()?.ask_note()?.handle();

    handler
        .figure_release_version()?
        .report()?
        .ask_about_new_package()?
        .ask_about_current_version()?
        .confirm_release()?
        .sync_with_remote()?
        .write_project()?
        .commit_changes()?
        .sync_with_remote()?
        .summon_registrator()?
        .revert_commit_maybe()?;

    // tag version and create release
    // NOTE: let's not do this for now
    Ok(())
}
