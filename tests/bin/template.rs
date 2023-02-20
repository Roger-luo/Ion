use super::*;
use anyhow::{Ok, Result};

#[test]
fn test_template() -> Result<()> {
    Ion::new().arg("template").arg("update").assert().success();
    Ion::new().arg("template").arg("list").assert().success();

    Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("--all")
        .assert()
        .success();

    Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("package")
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_nonce() -> Result<()> {
    let mut p = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(30_000))?;

    p.send_line("")?; // skip download if there is
                      // Send <ENTER> keycode to pty
    p.send_line("")?;
    p.exp_eof()?;

    Ok(())
}
