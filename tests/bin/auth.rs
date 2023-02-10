use super::*;
use anyhow::Result;

#[test]
fn test_login() -> Result<()> {
    let mut p = Ion::new()
        .arg("auth")
        .arg("login")
        .scratch()
        .spawn(Some(30_000))?;

    p.send_line("n")?; // don't launch browser
    p.exp_eof()?;
    Ok(())
}

#[test]
fn test_logout() -> Result<()> {
    Ion::new().arg("auth").arg("logout").assert().success();
    Ok(())
}
