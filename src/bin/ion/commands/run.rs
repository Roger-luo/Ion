use anyhow::format_err;
use clap::{arg, ArgMatches, Command, ValueHint};
use ion::{errors::CliResult, script::Script};

pub fn cli() -> Command {
    Command::new("run")
        .about("Run a script")
        .arg(
            arg!(sysimage: -J --sysimage [PATH] "Path to the sysimage to use")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            arg!(threads: -t --threads [NUM] "Number of threads to use")
                .value_hint(ValueHint::Other),
        )
        .arg(
            arg!(process: -p --procs [NUM] "Number of processes to use")
                .value_hint(ValueHint::Other),
        )
        .arg(arg!(quiet: -q --quiet "Quiet startup: no banner, suppress REPL warnings"))
        .arg(arg!(color: --color [OPT] "Enable or disable color text"))
        .arg(
            arg!(<PATH> "Path to the script to run")
                .trailing_var_arg(true)
                .num_args(1..),
        )
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let args: Vec<_> = match matches.get_many::<String>("PATH") {
        Some(paths) => paths.collect(),
        None => return Err(format_err!("No path provided").into()),
    };

    if args.len() == 0 {
        return Err(format_err!("No path provided").into());
    }

    let path = args[0].clone();
    let args = &args[1..];
    let mut cmd = Script::from_path(path.as_str())?.cmd();

    if let Some(path) = matches.get_one::<String>("sysimage") {
        cmd.arg(format!("--sysimage={}", path));
    }

    if let Some(path) = matches.get_one::<String>("threads") {
        cmd.arg(format!("--threads={}", path));
    }

    if let Some(path) = matches.get_one::<String>("process") {
        cmd.arg(format!("--procs={}", path));
    }

    if matches.get_flag("quiet") {
        cmd.arg("--quiet");
    }

    if let Some(opt) = matches.get_one::<String>("color") {
        cmd.arg(format!("--color={}", opt));
    }

    cmd.arg("--");
    cmd.arg(path);
    cmd.args(args);
    log::debug!("Running script: {:?}", cmd);
    let p = cmd.status()?;
    if p.success() {
        Ok(())
    } else {
        Err(format_err!("Script exited with non-zero status code").into())
    }
}
