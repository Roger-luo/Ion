use super::*;
use anyhow::Result;

#[test]
fn test_clone() -> Result<()> {
    let mut p = Ion::new()
        .arg("clone")
        .arg("Example")
        .arg("-f")
        .scratch()
        .spawn(Some(30_000))?;

    p.send_line("n")?;
    p.send_line("n")?;
    p.exp_eof()?;
    p.success()
}
