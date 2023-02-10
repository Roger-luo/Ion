pub mod auth;
pub mod clone;
pub mod new;
pub mod pkg;
pub mod script;

pub mod utils;
pub use utils::*;

#[test]
fn test_cli_help() {
    Ion::new().arg("help").assert().success();
}
