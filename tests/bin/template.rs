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
    let mut ps = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(5_000))?;

    ps.exp_string("Installed templates are:")?;

    // Send <ENTER> keycode to pty
    ps.send_control('j')?;
    ps.exp_string("name")?;
    ps.exp_eof()?;

    Ok(())
}
