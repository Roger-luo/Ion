//! PTY-based scenario tests for `<request-tool>` handling.

use std::time::Duration;

use scenario::{Scenario, Terminal};

const AUTOTUNE: &str = env!("CARGO_BIN_EXE_autotune");

fn autotune() -> Scenario {
    Scenario::new(AUTOTUNE).timeout(Duration::from_secs(10))
}

/// Once a tool is approved, a duplicate `<request-tool>` in a later turn
/// must NOT re-prompt the user — it is silently skipped.
///
/// Setup:
///   - Turn 1 contains `<request-tool>Bash</request-tool>`.
///   - PTY sends "y" to approve Bash.
///   - Turn 2 emits `<request-tool>Bash</request-tool>` again.
///
/// Expected:
///   - Only one "Allow tool 'Bash'?" prompt appears (turn 1).
///   - Turn 2 produces no prompt.
///   - The run completes without error.
#[test]
fn already_granted_tool_is_not_re_prompted() {
    let mut session = autotune()
        .env(
            "MOCK_RESPONSE_1",
            "research output <request-tool>Bash</request-tool>",
        )
        .env(
            "MOCK_RESPONSE_2",
            "more output <request-tool>Bash</request-tool>",
        )
        .terminal(Terminal::pty(80, 24))
        .spawn()
        .unwrap();

    // Turn 1: expect the approval prompt and approve.
    session.expect("Allow tool 'Bash'?").unwrap();
    session.send_line("y").unwrap();
    session.expect("Granted: Bash").unwrap();

    // Turn 2: the same tool is requested again — no second prompt.
    session.expect("--- turn 2 ---").unwrap();

    // The second turn should NOT show another approval prompt.
    // It should proceed directly to Done.
    session.expect("Done.").unwrap();

    // Verify the prompt did NOT appear a second time.
    let full_output = session.current_output();
    let prompt_count = full_output.matches("Allow tool 'Bash'?").count();
    assert_eq!(
        prompt_count, 1,
        "Expected exactly one approval prompt, found {prompt_count} in output:\n{full_output}"
    );

    let output = session.wait().unwrap();
    assert!(output.success(), "Process should exit successfully");
}
