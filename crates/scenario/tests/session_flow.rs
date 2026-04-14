use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use scenario::{Error, Key, Scenario, SessionConfig, Terminal};

fn session_flow_guard() -> MutexGuard<'static, ()> {
    static SESSION_FLOW_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    SESSION_FLOW_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap()
}

fn spawn(script: &str) -> scenario::Session {
    Scenario::new("sh")
        .args(["-c", script])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap()
}

#[test]
fn session_config_applies_shared_terminal_and_timeout_settings() {
    let _guard = session_flow_guard();
    let config = SessionConfig::pty(72, 20).timeout(Duration::from_secs(1));

    let mut session = Scenario::new("sh")
        .args(["-c", "stty size; printf ready\\n; sleep 5"])
        .session_config(&config)
        .spawn()
        .unwrap();

    session.expect("20 72").unwrap();
    session.expect("ready").unwrap();

    let started = Instant::now();
    let result = session.wait();

    assert!(matches!(result, Err(Error::Timeout(_))));
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn send_keys_submits_multiple_terminal_keys_in_order() {
    let _guard = session_flow_guard();
    let mut session =
        spawn("stty raw -echo; printf 'ready\\n'; dd bs=1 count=7 status=none | od -An -tx1");

    session.expect("ready").unwrap();
    session
        .send_keys([Key::Down, Key::Down, Key::Enter])
        .unwrap();
    session
        .expect_regex(r"1b\s+5b\s+42.*1b\s+5b\s+42.*0d")
        .unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn enter_and_ctrl_c_match_common_terminal_actions() {
    let _guard = session_flow_guard();
    let mut session = spawn(
        "trap 'printf trapped\\n; exit 130' INT; printf ready\\n; IFS= read -r line; printf '%s' \"$line\" | od -An -tx1; while :; do sleep 1; done",
    );

    session.expect("ready").unwrap();
    session.press(Key::Char('o')).unwrap();
    session.press(Key::Char('k')).unwrap();
    session.enter().unwrap();
    session.expect_regex(r"6f\s+6b").unwrap();

    session.ctrl_c().unwrap();
    session.expect("trapped").unwrap();

    let output = session.wait().unwrap();
    assert_eq!(output.exit_code(), 130);
}

#[test]
fn expect_screen_checks_visible_state_after_redraw() {
    let _guard = session_flow_guard();
    let mut session =
        spawn(r#"printf 'Option A'; printf '\r\033[K'; printf '> Option B'; sleep 0.1"#);

    session.expect("> Option B").unwrap();
    session.expect_screen("> Option B").unwrap();
    session.expect_screen_regex(r"> Option B").unwrap();
    session.expect_screen_not("Option A").unwrap();
    session.expect_screen_not_regex(r"Option A").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_screen_not_monitors_through_the_quiet_period() {
    let _guard = session_flow_guard();
    let session =
        spawn(r#"printf 'clean'; sleep 0.1; printf '\r\033[Kforbidden'; sleep 0.1; exit 0"#);

    let result = session.expect_screen_not("forbidden");

    assert!(matches!(result, Err(Error::UnexpectedPattern { .. })));

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_screen_not_timeout_uses_the_requested_quiet_period() {
    let _guard = session_flow_guard();
    let session = spawn(r#"printf 'clean'; sleep 0.08; exit 0"#);
    let started = std::time::Instant::now();

    session
        .expect_screen_not_timeout("forbidden", Duration::from_millis(50))
        .unwrap();

    assert!(started.elapsed() < Duration::from_millis(200));

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_screen_not_regex_timeout_uses_the_requested_quiet_period() {
    let _guard = session_flow_guard();
    let session = spawn(r#"printf 'clean'; sleep 0.08; exit 0"#);
    let started = std::time::Instant::now();

    session
        .expect_screen_not_regex_timeout(r"forbidden", Duration::from_millis(50))
        .unwrap();

    assert!(started.elapsed() < Duration::from_millis(200));

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_observes_settled_terminal_output() {
    let _guard = session_flow_guard();
    let mut session = spawn(r#"printf 'loading'; sleep 0.1; printf '\r\033[Kdone'; sleep 0.1"#);

    session.expect("done").unwrap();
    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();

    assert_eq!(session.visible_text().lines().next().unwrap_or(""), "done");

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_succeeds_for_a_blank_stable_screen() {
    let _guard = session_flow_guard();
    let session = spawn(r#"printf 'temp'; printf '\r\033[K'; sleep 1"#);
    let started = std::time::Instant::now();

    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(session.visible_text().lines().all(|line| line.is_empty()));

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_succeeds_for_an_already_stable_running_screen() {
    let _guard = session_flow_guard();
    let session = spawn(r#"printf 'steady'; sleep 1"#);
    let started = std::time::Instant::now();

    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(
        session.visible_text().lines().next().unwrap_or(""),
        "steady"
    );

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_does_not_return_before_the_first_redraw() {
    let _guard = session_flow_guard();
    let session = spawn(r#"sleep 0.2; printf 'late'; sleep 0.05; exit 0"#);
    let started = std::time::Instant::now();

    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(150));
    assert_eq!(session.visible_text().lines().next().unwrap_or(""), "late");

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_waits_out_the_quiet_period_after_final_redraw_even_if_process_exits() {
    let _guard = session_flow_guard();
    let session = spawn(r#"sleep 0.05; printf 'done'; sleep 0.05; exit 0"#);
    let started = std::time::Instant::now();

    session
        .wait_for_screen_stable(Duration::from_millis(200))
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(240));
    assert_eq!(session.visible_text().lines().next().unwrap_or(""), "done");

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn expect_screen_returns_immediately_after_process_exits_without_match() {
    let _guard = session_flow_guard();
    let session = spawn("printf 'ready\\n'; exit 0");
    let started = std::time::Instant::now();

    let result = session.expect_screen("never appears");

    assert!(matches!(result, Err(Error::ExpectTimeout { .. })));
    assert!(started.elapsed() < Duration::from_secs(1));

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn wait_for_screen_stable_times_out_when_screen_keeps_changing() {
    let _guard = session_flow_guard();
    let mut session = spawn(
        r#"trap 'exit 0' INT; i=0; printf 'start\n'; while :; do printf '\rframe %s' "$i"; i=$((i + 1)); sleep 0.02; done"#,
    );

    let result = session.wait_for_screen_stable(Duration::from_millis(200));

    assert!(matches!(result, Err(Error::Timeout(_))));

    session.ctrl_c().unwrap();
    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn screen_helpers_replace_current_output_slicing_for_redraw_flows() {
    let _guard = session_flow_guard();
    let mut session = spawn(
        r#"printf 'What metric?\n> Runtime performance\n  Memory usage';
           sleep 0.1;
           printf '\033[2A\r\033[J';
           printf 'How should we measure?\n> Benchmark runtime\n  Track memory';
           sleep 0.1"#,
    );

    session.expect("How should we measure").unwrap();
    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();
    session.expect_screen("How should we measure").unwrap();
    session.expect_screen_not("Runtime performance").unwrap();
    session.expect_screen("Benchmark runtime").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}
