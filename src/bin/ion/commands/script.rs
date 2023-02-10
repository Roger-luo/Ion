use clap::parser::ArgMatches;
use clap::{arg, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::script::{env_dir, Script};
use std::path::Path;
use std::path::PathBuf;

pub fn cli() -> Command {
    Command::new("script")
        .about("script tools")
        .subcommand(
            Command::new("update")
                .about("update the scripts environment by re-initalizing the environment")
                .arg(arg!(verbose: -v --verbose "Verbose mode"))
                .arg(arg!(<PATH> "The path of the script").value_hint(ValueHint::FilePath)),
        )
        .subcommand(
            Command::new("rm")
                .about("remove a script environment")
                .arg(arg!(<PATH> "The path of the script").value_hint(ValueHint::FilePath)),
        )
        .subcommand(
            Command::new("repl")
                .about("start a REPL from the script environment")
                .arg(arg!(<PATH> "The path of the script").value_hint(ValueHint::FilePath)),
        )
        .arg_required_else_help(true)
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    match matches.subcommand() {
        Some(("update", submatches)) => {
            let path = submatches.get_one::<String>("PATH").unwrap();
            remove_old_environment(&PathBuf::from(path))?;
            Script::from_path(config, path, submatches.get_flag("verbose"))?;
        }
        Some(("rm", submatches)) => {
            let path = submatches.get_one::<String>("PATH").unwrap();
            remove_old_environment(&PathBuf::from(path))?;
        }
        Some(("repl", submatches)) => {
            let path = submatches.get_one::<String>("PATH").unwrap();
            let env = env_dir(path)?;
            log::debug!("starting REPL from {}", env.display());
            std::process::Command::new("julia")
                .arg(format!("--project={}", env.display()))
                .spawn()?
                .wait()?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn remove_old_environment(path: &Path) -> CliResult {
    let env = env_dir(path.to_str().expect("invalid path"))?;
    if env.exists() {
        std::fs::remove_dir_all(env)?;
    }
    Ok(())
}
