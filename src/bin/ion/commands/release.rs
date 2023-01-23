use clap::parser::ArgMatches;
use clap::{arg, Command};
use colorful::Colorful;
use dialoguer::Confirm;
use ion::errors::CliResult;
use ion::release::handler::ReleaseHandler;
use ion::release::version_spec::VersionSpec;
use ion::utils::{current_project, git};
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("release")
        .about("release a new version of a package")
        .arg(arg!(<VERSION> "The version to release"))
        .arg(arg!([PATH] "The path of the package"))
        .arg(arg!(--branch [BRANCH] "The branch to release"))
        .arg(arg!(--registry [REGISTRY] "The registry to release").default_value("General"))
        .arg_required_else_help(true)
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let version = match matches.get_one::<String>("VERSION") {
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

    let branch = match matches.get_one::<String>("branch") {
        Some(branch) => branch.to_owned(),
        None => git::current_branch(&path.parent().unwrap().to_path_buf())?,
    };

    let registry_name = match matches.get_one::<String>("registry") {
        Some(registry) => registry.to_owned(),
        None => "General".to_owned(),
    };

    let mut release = ReleaseHandler::new(version, registry_name);
    release
        .path(path)?
        .branch(branch)
        .update_version()?
        .report()?;
    let mut dont_ask_again = false;
    if release.not_registered()? {
        if !Confirm::new()
            .with_prompt("do you want to register this version?")
            .interact()?
        {
            return Err(anyhow::format_err!("release cancelled").into());
        } else {
            dont_ask_again = true;
        }
    }

    if release.current_larger_than_latest()? {
        if release.is_current_continuously_greater()? {
            // confirm from user
            // update release version to current
            // print report again
            eprintln!(
                "{}: current version ({}) is a valid release version",
                "warning".yellow().bold(),
                release.get_version()?,
            );
            if Confirm::new()
                .with_prompt("do you want to release current version?")
                .interact()?
            {
                release.set_release_version(release.get_version()?.to_owned());
                release.report()?;
            } else {
                return Err(anyhow::format_err!("release cancelled").into());
            }
        } else {
            return Err(anyhow::format_err!("current version is not a registered version").into());
        }
    }

    if !dont_ask_again
        && !Confirm::new()
            .with_prompt("do you want to release this version?")
            .default(true)
            .interact()?
    {
        return Err(anyhow::format_err!("release cancelled").into());
    }

    // sync with remote
    release
        .sync_with_remote()?
        .write_project()?
        .commit_changes()?
        .sync_with_remote()?;

    // communicate using commit comment
    match release.summon_registrator() {
        Ok(_) => {
            eprintln!("{}: registrator summoned", "success".green().bold(),);
        }
        Err(_) => {
            release.revert_commit()?;
            return Err(anyhow::format_err!("registrator not summoned").into());
        }
    }
    // tag version and create release
    // NOTE: let's not do this for now
    Ok(())
}
