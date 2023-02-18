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
        .spawn(Some(30_000))?;

    p.exp_string("template was not found")?;
    p.send_control('c')?;
    p.exp_eof()?;

    Ok(())?;

    // Test nonce input
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
