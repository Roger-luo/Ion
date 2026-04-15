//! Minimal test harness for `<request-tool>` PTY scenario tests.
//!
//! Reads mock agent responses from environment variables `MOCK_RESPONSE_1`,
//! `MOCK_RESPONSE_2`, … and processes each sequentially through a
//! [`ToolSession`].  For each unapproved tool it prompts the user on the
//! terminal; already-approved tools are silently skipped.
//!
//! This binary exists solely to exercise the approval flow in PTY-based
//! integration tests.

use std::io::{Write, stdin, stdout};

use autotune::ToolSession;

fn main() {
    let mut session = ToolSession::new();

    // Collect MOCK_RESPONSE_1, MOCK_RESPONSE_2, … until the first missing key.
    let mut responses = Vec::new();
    for i in 1.. {
        match std::env::var(format!("MOCK_RESPONSE_{i}")) {
            Ok(val) => responses.push(val),
            Err(_) => break,
        }
    }

    if responses.is_empty() {
        eprintln!("No MOCK_RESPONSE_* environment variables set.");
        std::process::exit(1);
    }

    let mut reader = stdin().lock();
    let mut writer = stdout().lock();

    for (idx, response) in responses.iter().enumerate() {
        writeln!(writer, "--- turn {} ---", idx + 1).unwrap();
        writer.flush().unwrap();

        match session.process_response(response, &mut reader, &mut writer) {
            Ok(newly) => {
                if !newly.is_empty() {
                    writeln!(writer, "Granted: {}", newly.join(", ")).unwrap();
                }
            }
            Err(e) => {
                eprintln!("Error processing response: {e}");
                std::process::exit(1);
            }
        }
    }

    writeln!(writer, "Done.").unwrap();
}
