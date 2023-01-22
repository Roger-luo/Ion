use clap::parser::ArgMatches;
use ion::PackageSpec;

pub fn package_spec_list(matches: &ArgMatches) -> String {
    let packages = matches
        .get_many::<String>("PACKAGE")
        .into_iter()
        .flatten();

    packages.map(|p| PackageSpec::new(p))
        .map(|p| format!("{}", p))
        .collect::<Vec<_>>()
        .join(",")
}
