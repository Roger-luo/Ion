use clap::Command;
use clap::parser::ArgMatches;
use cargo::CliResult;
use crate::commands::pkg::JuliaCmd;

pub fn cli() -> Command {
    Command::new("gc")
        .about("garbage collect packages not used for a significant time")
        .arg_required_else_help(true)
}

pub fn exec(_: &ArgMatches) -> CliResult {
    "using Pkg; Pkg.gc()".as_julia_script()
}
