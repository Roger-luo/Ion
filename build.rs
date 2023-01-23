use copy_to_output::copy_to_output;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=resources/*");
    copy_to_output("resources", &env::var("PROFILE").unwrap()).expect("Could not copy");
}
