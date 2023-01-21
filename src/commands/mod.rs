use clap::{Command, ArgMatches};
use crate::errors::CliResult;

pub mod pkg;
pub mod new;

pub fn builtin() -> Vec<Command> {
    vec![
        new::cli(),
        pkg::add::cli(),
        pkg::develop::cli(),
        pkg::free::cli(),
        pkg::gc::cli(),
        pkg::precompile::cli(),
        pkg::remove::cli(),
        pkg::status::cli(),
        pkg::update::cli(),
    ]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&ArgMatches) -> CliResult> {
    let f = match cmd {
        "new" => new::exec,
        "add" => pkg::add::exec,
        "develop" => pkg::develop::exec,
        "free" => pkg::free::exec,
        "gc" => pkg::gc::exec,
        "precompile" => pkg::precompile::exec,
        "remove" => pkg::remove::exec,
        "status" => pkg::status::exec,
        "update" => pkg::update::exec,
        _ => return None,
    };
    Some(f)
}
