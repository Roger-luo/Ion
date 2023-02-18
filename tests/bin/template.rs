use super::*;
use anyhow::{Ok, Result};

#[test]
fn test_template() -> Result<()> {
    Ion::new().arg("template").arg("list").assert().success();

    Ion::new().arg("template").arg("update").assert().success();

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

    // Test escape from input / Ctrl+C behaviour with `ion template inspect` (cf. ask_inspect_input fn)
    let mut p = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(5_000))?;

    p.send_control('c')?;
    p.exp_eof()?;

    Ok(())?;

    // Test --verbose flag
    let mut pty = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("--verbose")
        .arg("package")
        .spawn(Some(5_000))?;

    pty.exp_string("Name:")?;
    pty.exp_string("Description:")?;
    pty.exp_string("Repo:")?;
    pty.exp_eof()?;

    Ok(())?;

    // Test --verbose flag, ask for user selection
    let mut ptysession = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("--verbose")
        .spawn(Some(10_000))?;

    // Send <ENTER> keycode
    ptysession.send_control('j')?;
    ptysession.exp_string("Name:")?;
    ptysession.exp_string("Description:")?;
    ptysession.exp_string("Repo:")?;
    ptysession.exp_eof()?;

    Ok(())?;

    // Test nonce input
    let mut ps = Ion::new()
        .arg("template")
        .arg("inspect")
        .arg("nonce")
        .spawn(Some(10_000))?;

    ps.exp_string("Installed templates are:")?;

    // Send <ENTER> keycode to pty
    ps.send_control('j')?;
    ps.send_control('j')?;
    ps.exp_string("name")?;
    ps.exp_eof()?;

    Ok(())
}
