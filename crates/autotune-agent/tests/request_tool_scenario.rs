//! End-to-end scenario tests for the `<request-tool>` XML protocol.
//!
//! These tests exercise `parse_tool_request` through the
//! `parse-tool-request` CLI binary, verifying exit codes and
//! error output under controlled terminal conditions.

use std::time::Duration;

use scenario::Scenario;

const PARSE_BIN: &str = env!("CARGO_BIN_EXE_parse-tool-request");

fn parse_tool_request() -> Scenario {
    Scenario::new(PARSE_BIN).timeout(Duration::from_secs(10))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Missing <reason> — issue #136
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn scenario_missing_reason_exits_nonzero() {
    let output = parse_tool_request()
        .stdin(b"<request-tool><tool>Bash</tool></request-tool>".to_vec())
        .run()
        .unwrap();

    assert!(
        !output.success(),
        "should exit non-zero on missing <reason>"
    );
    assert_eq!(output.exit_code(), 1);
}

#[test]
fn scenario_missing_reason_shows_parse_error() {
    let output = parse_tool_request()
        .stdin(b"<request-tool><tool>Bash</tool></request-tool>".to_vec())
        .run()
        .unwrap();

    let stderr = output.stderr();
    assert!(
        stderr.contains("reason"),
        "stderr should mention missing reason: {stderr}"
    );
    assert!(
        stderr.contains("parse error"),
        "stderr should contain 'parse error': {stderr}"
    );
}

#[test]
fn scenario_missing_reason_no_approval_prompt() {
    let output = parse_tool_request()
        .stdin(b"<request-tool><tool>Bash</tool></request-tool>".to_vec())
        .run()
        .unwrap();

    let stdout = output.stdout();
    // No tool= output means no successful parse, so no approval prompt
    assert!(
        !stdout.contains("tool="),
        "stdout should not contain parsed tool output (no approval prompt): {stdout}"
    );
}

#[test]
fn scenario_missing_reason_no_panic() {
    let output = parse_tool_request()
        .stdin(b"<request-tool><tool>Bash</tool></request-tool>".to_vec())
        .run()
        .unwrap();

    let stderr = output.stderr();
    assert!(!stderr.contains("panic"), "should not panic: {stderr}");
    assert!(
        !stderr.contains("RUST_BACKTRACE"),
        "should not show backtrace hint: {stderr}"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Valid input — sanity check
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn scenario_valid_request_exits_zero() {
    let output = parse_tool_request()
        .stdin(
            b"<request-tool><tool>Bash</tool><reason>Need to run tests</reason></request-tool>"
                .to_vec(),
        )
        .run()
        .unwrap();

    assert!(output.success(), "should exit zero on valid input");
    assert!(output.stdout().contains("tool=Bash"));
    assert!(output.stdout().contains("reason=Need to run tests"));
}
