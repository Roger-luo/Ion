use std::fmt::Display;

use clap::parser::ArgMatches;
use ion::PackageSpec;

pub struct PackageSpecList {
    pub list: Vec<PackageSpec>,
}

impl PackageSpecList {
    pub fn new(matches: &ArgMatches) -> Self {
        let packages = matches.get_many::<String>("PACKAGE").into_iter().flatten();
        Self {
            list: packages.map(PackageSpec::new).collect::<Vec<_>>(),
        }
    }
}

impl Display for PackageSpecList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.list.is_empty() {
            write!(f, "")
        } else {
            write!(
                f,
                "[{}]",
                self.list
                    .iter()
                    .map(|p| format!("{p}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}
