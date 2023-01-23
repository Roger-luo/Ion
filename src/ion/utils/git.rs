use super::*;
use anyhow::{format_err, Error};
use std::path::PathBuf;
use std::process::{Command, Output};

pub fn get_toplevel_path(path: &PathBuf) -> Result<PathBuf, Error> {
    let raw = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(path)
        .read_command()?;
    let path = PathBuf::from(raw);
    Ok(normalize_path(path.as_path()))
}

pub fn current_branch(path: &PathBuf) -> Result<String, Error> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(path)
        .read_command()
}

pub fn isdirty(path: &PathBuf) -> Result<bool, Error> {
    let p = Command::new("git")
        .arg("diff")
        .arg("--quiet")
        .arg("--exit-code")
        .current_dir(path)
        .status()?;
    Ok(!p.success())
}

pub fn isdirty_cached(path: &PathBuf) -> Result<bool, Error> {
    let p = Command::new("git")
        .arg("diff")
        .arg("--cached")
        .arg("--quiet")
        .arg("--exit-code")
        .current_dir(path)
        .status()?;
    Ok(!p.success())
}

pub fn commit(path: &PathBuf, msg: &str) -> Result<Output, Error> {
    let output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .current_dir(path)
        .output()?;

    if output.status.success() {
        Ok(output)
    } else {
        return Err(format_err!("Failed to commit"));
    }
}

pub fn pull(path: &PathBuf) -> Result<Output, Error> {
    let output = Command::new("git").arg("pull").current_dir(path).output()?;

    if output.status.success() {
        Ok(output)
    } else {
        return Err(format_err!("Failed to pull"));
    }
}

pub fn push(path: &PathBuf) -> Result<Output, Error> {
    let output = Command::new("git").arg("push").current_dir(path).output()?;

    if output.status.success() {
        Ok(output)
    } else {
        return Err(format_err!("Failed to push"));
    }
}
