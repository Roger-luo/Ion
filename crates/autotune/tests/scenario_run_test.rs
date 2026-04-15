use std::time::Duration;

use scenario::{Scenario, Terminal};

const AUTOTUNE: &str = env!("CARGO_BIN_EXE_autotune");

fn autotune(response: &str) -> Scenario {
    Scenario::new(AUTOTUNE)
        .arg(response)
        .terminal(Terminal::pty(120, 24))
        .timeout(Duration::from_secs(10))
}

#[test]
fn multiple_tool_requests_approve_first_deny_second() {
    let response = "\
        <request-tool><tool>Bash</tool><reason>a</reason></request-tool>\
        <request-tool><tool>WebFetch</tool><reason>b</reason></request-tool>";

    let mut session = autotune(response).spawn().unwrap();

    // First prompt — approve Bash
    session.expect("Allow tool \"Bash\"").unwrap();
    session.send_line("y").unwrap();

    // Second prompt — deny WebFetch
    session.expect("Allow tool \"WebFetch\"").unwrap();
    session.send_line("n").unwrap();

    // Summary lists what was granted vs denied
    session.expect("Granted: Bash").unwrap();
    session.expect("Denied: WebFetch").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn multiple_tool_requests_deny_all() {
    let response = "\
        <request-tool><tool>Bash</tool><reason>run build</reason></request-tool>\
        <request-tool><tool>WebFetch</tool><reason>download spec</reason></request-tool>";

    let mut session = autotune(response).spawn().unwrap();

    session.expect("Allow tool \"Bash\"").unwrap();
    session.send_line("n").unwrap();

    session.expect("Allow tool \"WebFetch\"").unwrap();
    session.send_line("n").unwrap();

    session.expect("Denied: Bash, WebFetch").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn multiple_tool_requests_approve_all() {
    let response = "\
        <request-tool><tool>Bash</tool><reason>a</reason></request-tool>\
        <request-tool><tool>WebFetch</tool><reason>b</reason></request-tool>\
        <request-tool><tool>Read</tool><reason>c</reason></request-tool>";

    let mut session = autotune(response).spawn().unwrap();

    session.expect("Allow tool \"Bash\"").unwrap();
    session.send_line("y").unwrap();

    session.expect("Allow tool \"WebFetch\"").unwrap();
    session.send_line("yes").unwrap();

    session.expect("Allow tool \"Read\"").unwrap();
    session.send_line("y").unwrap();

    session.expect("Granted: Bash, WebFetch, Read").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn single_tool_request_still_works() {
    let response = "<request-tool><tool>Bash</tool><reason>compile</reason></request-tool>";

    let mut session = autotune(response).spawn().unwrap();

    session.expect("Allow tool \"Bash\"").unwrap();
    session.send_line("y").unwrap();

    session.expect("Granted: Bash").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn no_tool_requests_reports_none() {
    let response = "<plan>Do something cool</plan>";

    let output = autotune(response).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("No tool requests found"));
}
