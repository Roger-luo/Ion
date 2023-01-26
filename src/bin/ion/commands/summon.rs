use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::errors::CliResult;
use ion::spec::JuliaProjectFile;
use ion::utils::current_project;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("summon")
        .about("summon JuliaRegistrator to register the package")
        .arg(
            arg!([PATH] "The path of the package")
                .value_hint(ValueHint::DirPath)
        )
        .arg(arg!(-b --branch [BRANCH] "The branch to release"))
        .arg(arg!(--"no-prompt" "Do not prompt for confirmation"))
        .arg(arg!(--"skip-note" "Skip interactive release note editing"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let path = match matches.get_one::<String>("PATH") {
        Some(path) => PathBuf::from(path),
        None => match current_project(std::env::current_dir()?) {
            Some(path) => path,
            None => return Err(anyhow::format_err!("cannot find valid Project.toml").into()),
        },
    };

    let branch = matches.get_one::<String>("branch");

    log::debug!("summoning JuliaRegistrator to register {}", path.display());

    JuliaProjectFile::root_project(path)?
        .summon()?
        .branch(branch)
        .prompt(!matches.get_flag("no-prompt"))
        .summon(matches.get_flag("skip-note"))?;
    Ok(())
}
