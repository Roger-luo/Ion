use anyhow::Result;
use super::*;

#[test]
fn test_clone() -> Result<()> {
    let mut p = Ion::new()
        .arg("clone")
        .arg("Example")
        .arg("-f")
        .env("GITHUB_TOKEN", std::env::var("GITHUB_TOKEN")?)
        .scratch()
        .spawn(Some(30_000))?;

    p.send_line("n")?;
    p.exp_eof()?;
    Ok(())
}
