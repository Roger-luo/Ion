use clap::{ArgMatches, Command};
use ion::errors::CliResult;

pub mod auth;
pub mod clone;
pub mod new;
pub mod pkg;
pub mod release;
pub mod bump;
pub mod summon;

pub fn builtin() -> Vec<Command> {
    vec![
        auth::cli(),
        clone::cli(),
        release::cli(),
        summon::cli(),
        bump::cli(),
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
        "auth" => auth::exec,
        "clone" => clone::exec,
        "release" => release::exec,
        "summon" => summon::exec,
        "bump" => bump::exec,
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
