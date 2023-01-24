use anyhow::format_err;
use clap::parser::ArgMatches;
use clap::{arg, Command};
use ion::errors::CliResult;
use ion::utils::git;
use ion::Registry;
use std::path::PathBuf;
use url::Url;

pub fn cli() -> Command {
    Command::new("clone")
        .about("Clone a package from URL or registry")
        .arg(arg!(url_or_name: <URL> "The name/url of the package"))
        .arg(arg!(dest: [PATH] "The path of the package"))
        .arg(arg!(registry: --registry [REGISTRY] "The registry to use").default_value("General"))
}

pub fn exec(matches: &ArgMatches) -> CliResult {
    let url_or_name = matches.get_one::<String>("url_or_name").unwrap().to_owned();
    let dest = matches.get_one::<PathBuf>("dest");
    let registry_name = matches.get_one::<String>("registry").unwrap().to_owned();

    let (url, name) = match Url::parse(&*url_or_name) {
        Ok(url) => {
            let name: Option<String> = match url.path_segments() {
                Some(segments) => segments.last().and_then(|name| Some(name.to_string())),
                None => None,
            };
            (url, name)
        }
        Err(_) => {
            let url = Registry::read(registry_name)?
                .package()
                .name(url_or_name.to_owned())
                .get_url()?;
            (url, Some(url_or_name))
        }
    };

    match (name, dest) {
        (Some(_), Some(dest)) => {
            git::clone(url.as_str(), dest)?;
        }
        (Some(name), None) => {
            git::clone(url.as_str(), &PathBuf::from(name))?;
        }
        (None, Some(dest)) => {
            git::clone(url.as_str(), dest)?;
        }
        (None, None) => {
            return Err(format_err!("No name or destination provided").into());
        }
    }
    Ok(())
}
