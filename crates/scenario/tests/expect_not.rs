use scenario::{Error, Scenario, Terminal};

fn spawn_session(script: &str) -> scenario::Session {
    Scenario::new("sh")
        .args(["-c", script])
        .terminal(Terminal::pty(80, 24))
        .spawn()
        .unwrap()
}

#[test]
fn expect_not_ignores_already_seen_output() {
    let mut session = spawn_session("printf '%s\n' 'forbidden before marker' 'ready-marker'");

    session.expect("ready-marker").unwrap();
    session.expect_not("forbidden").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_not_fails_when_literal_pattern_appears() {
    let session = spawn_session("printf '%s\n' 'start forbidden end'");

    let result = session.expect_not("forbidden");
    assert!(matches!(
        result,
        Err(Error::UnexpectedPattern { pattern, buffer })
        if pattern == "forbidden" && buffer.contains("forbidden")
    ));

    let _ = session.wait();
}

#[test]
fn expect_not_regex_ignores_already_seen_output() {
    let mut session = spawn_session("printf '%s\n' 'digits 123 before marker' 'ready-marker'");

    session.expect("ready-marker").unwrap();
    session.expect_not_regex(r"\d+").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_not_regex_fails_when_pattern_matches() {
    let session = spawn_session("printf '%s\n' 'version 42'");

    let result = session.expect_not_regex(r"\d+");
    assert!(matches!(
        result,
        Err(Error::UnexpectedPattern { pattern, buffer })
        if pattern == r"\d+" && buffer.contains("42")
    ));

    let _ = session.wait();
}
