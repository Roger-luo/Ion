use clap::Command;
use clap::parser::ArgMatches;
use crate::errors::CliResult;
use crate::julia::Julia;

pub fn cli() -> Command {
    Command::new("gc")
        .about("garbage collect packages not used for a significant time")
        .arg_required_else_help(true)
}

pub fn exec(_: &ArgMatches) -> CliResult {
    "using Pkg; Pkg.gc()".julia_exec()
}
