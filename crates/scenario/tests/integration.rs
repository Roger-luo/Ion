use std::time::Duration;

use scenario::{Error, Scenario, Terminal};

// ── Piped mode ──────────────────────────────────────────────────────

#[test]
fn piped_echo() {
    let output = Scenario::new("echo").arg("hello world").run().unwrap();

    assert!(output.success());
    assert_eq!(output.exit_code(), 0);
    assert!(output.stdout().contains("hello world"));
}

#[test]
fn piped_exit_code() {
    let output = Scenario::new("sh").args(["-c", "exit 42"]).run().unwrap();

    assert!(!output.success());
    assert_eq!(output.exit_code(), 42);
}

#[test]
fn piped_stderr() {
    let output = Scenario::new("sh")
        .args(["-c", "echo error >&2"])
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stderr().contains("error"));
}

#[test]
fn piped_stdin() {
    let output = Scenario::new("cat")
        .stdin(b"hello from stdin".to_vec())
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stdout().contains("hello from stdin"));
}

#[test]
fn piped_env_var() {
    let output = Scenario::new("sh")
        .args(["-c", "echo $MY_TEST_VAR"])
        .env("MY_TEST_VAR", "scenario_test")
        .run()
        .unwrap();

    assert!(output.stdout().contains("scenario_test"));
}

#[test]
fn piped_current_dir() {
    let dir = std::env::temp_dir();
    let output = Scenario::new("pwd").current_dir(&dir).run().unwrap();

    // Resolve symlinks for comparison (macOS /tmp -> /private/tmp)
    let canonical = dir.canonicalize().unwrap();
    assert!(output.stdout().contains(canonical.to_str().unwrap()));
}

#[test]
fn piped_timeout() {
    let result = Scenario::new("sleep")
        .arg("60")
        .timeout(Duration::from_millis(200))
        .run();

    assert!(matches!(result, Err(Error::Timeout(_))));
}

#[test]
fn piped_display_output() {
    let output = Scenario::new("echo").arg("hello").run().unwrap();

    let display = output.to_string();
    assert!(display.contains("success: true"));
    assert!(display.contains("exit_code: 0"));
    assert!(display.contains("----- stdout -----"));
    assert!(display.contains("hello"));
}

// ── PTY mode ────────────────────────────────────────────────────────

#[test]
fn pty_echo() {
    let output = Scenario::new("echo")
        .arg("hello pty")
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stdout().contains("hello pty"));
}

#[test]
fn pty_exit_code() {
    let output = Scenario::new("sh")
        .args(["-c", "exit 7"])
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();

    assert!(!output.success());
    assert_eq!(output.exit_code(), 7);
}

#[test]
fn pty_env_var() {
    let output = Scenario::new("sh")
        .args(["-c", "echo $MY_PTY_VAR"])
        .env("MY_PTY_VAR", "pty_test")
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();

    assert!(output.stdout().contains("pty_test"));
}

#[test]
fn pty_stderr_merged() {
    // In PTY mode, stderr is merged into stdout
    let output = Scenario::new("sh")
        .args(["-c", "echo error >&2"])
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();

    assert!(output.stderr().is_empty());
    // The error output appears in stdout instead
    assert!(output.stdout().contains("error"));
}

// ── Interactive session ─────────────────────────────────────────────

#[test]
fn session_cat_echo() {
    let mut session = Scenario::new("cat")
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.send_line("hello").unwrap();
    session.expect("hello").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn session_expect_timeout() {
    let mut session = Scenario::new("cat")
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_millis(500))
        .spawn()
        .unwrap();

    let result = session.expect("this will never appear");
    assert!(matches!(result, Err(Error::ExpectTimeout { .. })));

    // Clean up
    let _ = session.wait();
}

#[test]
fn session_expect_regex() {
    let mut session = Scenario::new("sh")
        .args(["-c", "echo 'version 1.2.3'"])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.expect_regex(r"version \d+\.\d+\.\d+").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn spawn_requires_pty() {
    let result = Scenario::new("echo").spawn();
    assert!(matches!(result, Err(Error::SpawnRequiresPty)));
}

#[test]
fn session_current_output() {
    let mut session = Scenario::new("sh")
        .args(["-c", "echo 'some output'"])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.expect("some output").unwrap();
    let captured = session.current_output();
    assert!(captured.contains("some output"));

    let _ = session.wait();
}
