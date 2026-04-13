use std::time::Duration;

use scenario::{Key, Scenario, Terminal};

#[test]
fn send_key_down_writes_the_arrow_escape_sequence() {
    let mut session = Scenario::new("sh")
        .args([
            "-c",
            "printf 'ready\\n'; IFS= read -r line; printf '%s' \"$line\" | od -An -tx1",
        ])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.expect("ready").unwrap();
    session.send_key(Key::Down).unwrap();
    session.send_key(Key::Enter).unwrap();

    session.expect_regex(r"1b\s+5b\s+42").unwrap();
    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn send_key_backspace_edits_the_line_before_submit() {
    let mut session = Scenario::new("sh")
        .args([
            "-c",
            "printf 'ready\\n'; IFS= read -r line; printf '%s' \"$line\" | od -An -tx1",
        ])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.expect("ready").unwrap();
    session.send_key(Key::Char('a')).unwrap();
    session.send_key(Key::Char('b')).unwrap();
    session.send_key(Key::Backspace).unwrap();
    session.send_key(Key::Char('c')).unwrap();
    session.send_key(Key::Enter).unwrap();

    session.expect_regex(r"61\s+63").unwrap();
    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn send_key_ctrl_c_interrupts_the_foreground_process() {
    let mut session = Scenario::new("sh")
        .args([
            "-c",
            "trap 'printf trapped\\n; exit 130' INT; printf ready\\n; while :; do sleep 1; done",
        ])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap();

    session.expect("ready").unwrap();
    session.send_key(Key::CtrlC).unwrap();
    session.expect("trapped").unwrap();

    let output = session.wait().unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 130);
}
