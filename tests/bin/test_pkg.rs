use assert_cmd::Command;
use std::{env, path::PathBuf, fs::create_dir_all};

fn setup() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let scratch = PathBuf::from(root).join("tests").join("packages").join("scratch");
    create_dir_all(scratch.clone()).unwrap();
    env::set_current_dir(scratch).unwrap();
}

#[test]
fn test_cli_example() {
    setup();
    Command::cargo_bin("ion").unwrap()
        .arg("help").assert().success();
    Command::cargo_bin("ion").unwrap()
        .arg("clone").arg("Example").assert().success();
    // Command::cargo_bin("ion").unwrap()
    //     .arg("new").arg("TestNew").arg("--template=package").assert().success();
}
