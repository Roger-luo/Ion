use super::*;

#[test]
fn test_script() {
    Ion::new()
        .arg("run")
        .arg("script.jl")
        .packages()
        .assert()
        .success();

    Ion::new()
        .arg("script")
        .arg("rm")
        .arg("script.jl")
        .packages()
        .assert()
        .success();
}
