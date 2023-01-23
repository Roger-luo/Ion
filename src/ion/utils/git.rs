use std::path::PathBuf;
use std::process::Command;
use anyhow::{format_err, Error};
use super::*;

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
        .read_command().into()
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

pub fn commit(path: &PathBuf, msg: &str) -> Result<(), Error> {
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .current_dir(path)
        .status()?;
    Ok(())
}

pub fn pull(path: &PathBuf) -> Result<(), Error> {
    let p = Command::new("git")
        .arg("pull")
        .current_dir(path)
        .status()?;
    
    if p.success() {
        return Ok(())
    } else {
        return Err(format_err!("Failed to pull"))
    }
}

pub fn push(path: &PathBuf) -> Result<(), Error> {
    let p = Command::new("git")
        .arg("push")
        .current_dir(path)
        .status()?;
    
    if p.success() {
        return Ok(())
    } else {
        return Err(format_err!("Failed to push"))
    }
}
