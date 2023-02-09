use clap::{crate_authors, crate_description, crate_version, ArgMatches, Command};
use ion::errors::{CliError, CliResult};
use ion::config::Config;

pub mod commands;

fn cli() -> Command {
    Command::new("ion")
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommands(commands::builtin())
}

fn main() {
    env_logger::init();
    let matches = cli().get_matches();
    let result = exec(&matches);

    if let Err(err) = result {
        if let Some(ref err) = err.error {
            if let Some(clap_err) = err.downcast_ref::<clap::Error>() {
                let exit_code = i32::from(clap_err.use_stderr());
                let _ = clap_err.print();
                std::process::exit(exit_code)
            }
        }

        let CliError { error, exit_code } = err;
        if let Some(error) = error {
            // display_error(&error, shell);
            eprintln!("{error}");
        }
        std::process::exit(exit_code)
    }
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let mut config = Config::read()?;
    let (cmd, subcommand_args) = match matches.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            cli().print_help()?;
            return Ok(());
        }
    };

    execute_subcommand(cmd, &mut config, subcommand_args)
}

fn execute_subcommand(cmd: &str, config: &mut Config, matches: &ArgMatches) -> CliResult {
    if let Some(exec) = commands::builtin_exec(cmd) {
        return exec(config, matches);
    }
    Err(CliError::new(
        anyhow::format_err!("unknown subcommand: {}", cmd),
        1,
    ))
}
