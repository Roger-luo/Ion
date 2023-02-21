use anyhow::Result;
use cargo_metadata::MetadataCommand;
use clap::{ArgMatches, Command};
use toml_edit::Document;

pub fn cli() -> Command {
    Command::new("version").about("Print the version of the current crate")
}

pub fn exec(_matches: &ArgMatches) -> Result<()> {
    let metadata_cmd = MetadataCommand::new();
    let metadata = metadata_cmd.exec()?;
    let root_package = metadata.root_package().unwrap();
    let manifest = root_package.manifest_path.to_owned();
    let source = std::fs::read_to_string(manifest.to_owned())?;
    let doc = source.parse::<Document>()?;

    if let Some(version) = doc["package"]["version"].as_str() {
        println!("{version}");
    }

    Ok(())
}
