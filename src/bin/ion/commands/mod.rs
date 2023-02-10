pub mod auth;
pub mod bump;
pub mod clone;
pub mod completions;
pub mod new;
pub mod pkg;
pub mod release;
pub mod run;
pub mod script;
pub mod summon;
pub mod template;

pub use clap::parser::ArgMatches;
pub use clap::{arg, Command, ValueHint};
pub use ion::config::Config;
pub use ion::errors::CliResult;
pub use ion::utils::{assert_julia_version, Julia};
pub use pkg::PackageSpecList;

pub fn builtin() -> Vec<Command> {
    vec![
        auth::cli(),
        clone::cli(),
        release::cli(),
        summon::cli(),
        bump::cli(),
        new::cli(),
        run::cli(),
        script::cli(),
        pkg::add::cli(),
        pkg::develop::cli(),
        pkg::free::cli(),
        pkg::gc::cli(),
        pkg::precompile::cli(),
        pkg::remove::cli(),
        pkg::status::cli(),
        pkg::update::cli(),
        pkg::why::cli(),
        completions::cli(),
        template::cli(),
    ]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&mut Config, &ArgMatches) -> CliResult> {
    let f = match cmd {
        "auth" => auth::exec,
        "clone" => clone::exec,
        "release" => release::exec,
        "summon" => summon::exec,
        "bump" => bump::exec,
        "new" => new::exec,
        "run" => run::exec,
        "script" => script::exec,
        "add" => pkg::add::exec,
        "develop" => pkg::develop::exec,
        "free" => pkg::free::exec,
        "gc" => pkg::gc::exec,
        "precompile" => pkg::precompile::exec,
        "remove" => pkg::remove::exec,
        "status" => pkg::status::exec,
        "update" => pkg::update::exec,
        "why" => pkg::why::exec,
        "completions" => completions::exec,
        "template" => template::exec,
        _ => return None,
    };
    Some(f)
}
