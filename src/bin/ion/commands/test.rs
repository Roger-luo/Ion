use clap::{arg, ArgMatches, Command, ValueHint};
use ion::config::Config;
use ion::errors::CliResult;
use ion::test::JuliaTestRunner;

pub fn cli() -> Command {
    Command::new("test")
        .about("Run tests")
        .arg(
            arg!([PATH] "path to tests")
                .value_hint(ValueHint::DirPath)
                .default_value("test"),
        )
        .arg(arg!(--coverage "collect coverage information"))
        .arg(arg!(--color [COLOR] "colorize the output"))
}

pub fn exec(config: &mut Config, matches: &ArgMatches) -> CliResult {
    let default_test_dir = "test".to_string();
    let path = matches
        .get_one::<String>("PATH")
        .unwrap_or(&default_test_dir);
    let coverage = matches.get_flag("coverage");
    let color = matches.get_one::<String>("color");

    let mut runner = JuliaTestRunner::new(path)?;
    match color {
        Some(color) => runner.arg(format!("--color={}", color)),
        None => runner.arg("--color=yes"),
    }

    if coverage {
        runner.arg("--code-coverage=user");
    }

    let report = runner.run(config)?;
    println!("{}", report);
    Ok(())
}
