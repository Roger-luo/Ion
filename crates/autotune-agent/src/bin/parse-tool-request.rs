/// Minimal CLI that reads a `<request-tool>` fragment from stdin,
/// parses it, and exits non-zero on error.
///
/// Used by scenario tests to verify end-to-end parse-error behaviour
/// without requiring the full autotune runtime.
use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    match autotune_agent::protocol::parse_tool_request(&input) {
        Ok(req) => {
            println!("tool={}", req.tool);
            println!("reason={}", req.reason);
            if let Some(scope) = &req.scope {
                println!("scope={scope}");
            }
        }
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    }
}
