use clap::Command;

mod bump;

fn main() -> Result<(), anyhow::Error> {
    let app = Command::new("xtask")
        .about("A task runner for the xtask crate")
        .subcommand(bump::cli())
        .arg_required_else_help(true);
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("bump", submatches)) => bump::exec(submatches),
        _ => Ok(()),
    }
}
