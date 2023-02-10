use assert_cmd::Command;
use super::setup;

#[test]
fn test_cli_example() {
    setup();
    Command::cargo_bin("ion")
        .unwrap()
        .arg("help")
        .assert()
        .success();
}

#[test]
fn test_add() {
    setup();
    Command::cargo_bin("ion")
        .unwrap()
        .current_dir("A")
        .arg("add")
        .arg("Example")
        .assert()
        .success();
}
