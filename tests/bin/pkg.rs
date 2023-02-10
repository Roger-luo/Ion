use super::*;

#[test]
fn test_add_rm() {
    Ion::new()
        .arg("add")
        .arg("Example")
        .packages_dir("A")
        .assert()
        .success();
    Ion::new()
        .arg("rm")
        .arg("Example")
        .packages_dir("A")
        .assert()
        .success();
}

#[test]
fn test_develop() {
    Ion::new()
        .arg("develop")
        .arg("lib/Bar")
        .packages_dir("B")
        .assert()
        .success();
}
