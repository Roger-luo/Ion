use std::time::Duration;

use scenario::{Scenario, Terminal};

#[test]
fn visible_screen_keeps_only_the_latest_single_line_redraw() {
    let mut session = Scenario::new("sh")
        .args([
            "-c",
            r#"printf 'Option A'; printf '\r\033[K'; printf '> Option B'; sleep 0.1"#,
        ])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(2))
        .spawn()
        .unwrap();

    session.expect("> Option B").unwrap();

    let current_output = session.current_output();
    assert!(current_output.contains("Option A"));
    assert!(current_output.contains("> Option B"));

    let screen = session.visible_screen();
    assert_eq!(screen[0], "> Option B");
    assert!(screen[1..].iter().all(|line| line.is_empty()));
}

#[test]
fn visible_screen_reflects_in_place_multiline_redraws() {
    let mut session = Scenario::new("sh")
        .args([
            "-c",
            r#"printf 'top\nmiddle\nbottom'; printf '\033[1A\r\033[K'; printf 'middle updated'; sleep 0.1"#,
        ])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(2))
        .spawn()
        .unwrap();

    session.expect("middle updated").unwrap();

    let current_output = session.current_output();
    assert!(current_output.contains("middle\n"));
    assert!(current_output.contains("middle updated"));

    let screen = session.visible_screen();
    assert_eq!(screen[0], "top");
    assert_eq!(screen[1], "middle updated");
    assert_eq!(screen[2], "bottom");
    assert!(screen[3..].iter().all(|line| line.is_empty()));
}
