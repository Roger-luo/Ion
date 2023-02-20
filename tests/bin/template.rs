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
fn test_ctrl_c() -> Result<()> {
    // Test escape from input / Ctrl+C behaviour with `ion template inspect` (cf. ask_inspect_input fn)
    let mut p = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(5_000))?;

    p.send_control('j')?; // skip download if there is
    p.read_line()?; // The nonce template was not found.
    p.read_line()?; // Installed templates are:
    p.read_line()?; // research
    p.read_line()?; // project
    p.read_line()?; // package
    p.send_control('c')?;
    p.exp_eof()?;

    Ok(())
}

#[test]
fn test_nonce() -> Result<()> {
    let mut p = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(5_000))?;

    p.send_control('j')?; // skip download if there is
    p.exp_string("Installed templates are:")?;

    // Send <ENTER> keycode to pty
    p.send_control('j')?;
    p.exp_string("name")?;
    p.exp_eof()?;

    Ok(())
}
