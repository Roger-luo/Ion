use clap::Command;

mod bump;
mod version;

fn main() -> Result<(), anyhow::Error> {
    let app = Command::new("xtask")
        .about("A task runner for the xtask crate")
        .subcommands(vec![
            bump::cli(), version::cli(),
        ])
        .arg_required_else_help(true);
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("bump", submatches)) => bump::exec(submatches),
        Some(("version", submatches)) => version::exec(submatches),
        _ => Ok(()),
    }
}
