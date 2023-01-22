use env_logger;
use clap::{Command, ArgMatches};
use ion::errors::{CliError, CliResult};

pub mod commands;

fn cli() -> Command {
    Command::new("ion")
        .about("The ion package manager for Julia")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommands(commands::builtin())
}

fn main() {
    env_logger::init();
    let matches = cli().get_matches();
    let result = exec(&matches);

    match result {
        Err(err) => {
            if let Some(ref err) = err.error {
                if let Some(clap_err) = err.downcast_ref::<clap::Error>() {
                    let exit_code = if clap_err.use_stderr() { 1 } else { 0 };
                    let _ = clap_err.print();
                    std::process::exit(exit_code)
                }
            }

            let CliError { error, exit_code } = err;
            if let Some(error) = error {
                // display_error(&error, shell);
                eprintln!("{}", error);
            }
            std::process::exit(exit_code)
        },
        Ok(()) => {}
    }
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let (cmd, subcommand_args) = match matches.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            cli().print_help()?;
            return Ok(());
        }
    };

    execute_subcommand(cmd, subcommand_args)
}

fn execute_subcommand(cmd: &str, matches: &ArgMatches) -> CliResult {
    if let Some(exec) = commands::builtin_exec(cmd) {
        return exec(matches);
    }
    Err(CliError::new(
        anyhow::format_err!("unknown subcommand: {}", cmd),1
    ))
}
