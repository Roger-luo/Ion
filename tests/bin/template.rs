use super::*;

#[test]
fn test_template() {
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
}
