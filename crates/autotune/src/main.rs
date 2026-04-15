use std::io::{BufReader, Write, stdin, stdout};

use autotune::{format_summary, parse_requests, prompt_requests};

fn main() {
    let response = std::env::args()
        .nth(1)
        .expect("usage: autotune <agent-response>");

    let requests = parse_requests(&response);
    if requests.is_empty() {
        println!("No tool requests found.");
        return;
    }

    let mut reader = BufReader::new(stdin().lock());
    let mut writer = stdout().lock();

    let reviewed =
        prompt_requests(&requests, &mut reader, &mut writer).expect("failed to read user input");

    writeln!(writer).unwrap();
    writeln!(writer, "{}", format_summary(&reviewed)).unwrap();
}
