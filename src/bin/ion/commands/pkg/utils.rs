use clap::parser::ArgMatches;
use ion::PackageSpec;

pub fn package_spec_list(matches: &ArgMatches) -> String {
    let packages = matches.get_many::<String>("PACKAGE").into_iter().flatten();

    packages
        .map(PackageSpec::new)
        .map(|p| format!("{p}"))
        .collect::<Vec<_>>()
        .join(",")
}
