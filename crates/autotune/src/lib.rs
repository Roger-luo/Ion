//! # autotune
//!
//! Parse and interactively approve `<request-tool>` fragments from agent
//! responses.
//!
//! A research-agent may emit one or more `<request-tool>` XML fragments in a
//! single turn. Each fragment names a tool and a reason for requesting it.
//! The CLI presents each request to the user, collects approve/deny decisions,
//! and produces a summary suitable for sending back to the agent.

use std::io::{BufRead, Write};

/// A single tool request parsed from a `<request-tool>` fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequest {
    pub tool: String,
    pub reason: String,
}

/// Outcome of a single tool-request decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Granted,
    Denied,
}

/// A tool request together with the user's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedRequest {
    pub request: ToolRequest,
    pub decision: Decision,
}

/// Parse all `<request-tool>` fragments from the given text.
///
/// Each fragment is expected to contain `<tool>…</tool>` and
/// `<reason>…</reason>` child elements.
pub fn parse_requests(input: &str) -> Vec<ToolRequest> {
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start) = input[search_from..].find("<request-tool>") {
        let abs_start = search_from + start;
        let after_open = abs_start + "<request-tool>".len();

        let Some(end) = input[after_open..].find("</request-tool>") else {
            break;
        };
        let inner = &input[after_open..after_open + end];

        let tool = extract_element(inner, "tool").unwrap_or_default();
        let reason = extract_element(inner, "reason").unwrap_or_default();

        if !tool.is_empty() {
            results.push(ToolRequest { tool, reason });
        }

        search_from = after_open + end + "</request-tool>".len();
    }

    results
}

/// Extract the text content of a simple XML element like `<tag>…</tag>`.
fn extract_element(input: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = input.find(&open)? + open.len();
    let end = input[start..].find(&close)? + start;
    Some(input[start..end].trim().to_string())
}

/// Prompt the user for each tool request, reading answers from `reader` and
/// writing prompts to `writer`. Returns the list of reviewed requests.
pub fn prompt_requests<R: BufRead, W: Write>(
    requests: &[ToolRequest],
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<Vec<ReviewedRequest>> {
    let mut reviewed = Vec::with_capacity(requests.len());

    for (i, req) in requests.iter().enumerate() {
        write!(
            writer,
            "[{}/{}] Allow tool \"{}\"? (reason: {}) [y/N] ",
            i + 1,
            requests.len(),
            req.tool,
            req.reason
        )?;
        writer.flush()?;

        let mut answer = String::new();
        reader.read_line(&mut answer)?;
        let granted = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");

        reviewed.push(ReviewedRequest {
            request: req.clone(),
            decision: if granted {
                Decision::Granted
            } else {
                Decision::Denied
            },
        });
    }

    Ok(reviewed)
}

/// Build a human-readable summary of granted vs denied requests.
pub fn format_summary(reviewed: &[ReviewedRequest]) -> String {
    let granted: Vec<&str> = reviewed
        .iter()
        .filter(|r| r.decision == Decision::Granted)
        .map(|r| r.request.tool.as_str())
        .collect();
    let denied: Vec<&str> = reviewed
        .iter()
        .filter(|r| r.decision == Decision::Denied)
        .map(|r| r.request.tool.as_str())
        .collect();

    let mut parts = Vec::new();
    if !granted.is_empty() {
        parts.push(format!("Granted: {}", granted.join(", ")));
    }
    if !denied.is_empty() {
        parts.push(format!("Denied: {}", denied.join(", ")));
    }
    if parts.is_empty() {
        "No tool requests.".to_string()
    } else {
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_request() {
        let input = "<request-tool><tool>Bash</tool><reason>run tests</reason></request-tool>";
        let reqs = parse_requests(input);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "Bash");
        assert_eq!(reqs[0].reason, "run tests");
    }

    #[test]
    fn parse_multiple_requests() {
        let input = "\
            <request-tool><tool>Bash</tool><reason>a</reason></request-tool>\
            <request-tool><tool>WebFetch</tool><reason>b</reason></request-tool>";
        let reqs = parse_requests(input);
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].tool, "Bash");
        assert_eq!(reqs[1].tool, "WebFetch");
    }

    #[test]
    fn parse_no_requests() {
        let reqs = parse_requests("just some text without any fragments");
        assert!(reqs.is_empty());
    }

    #[test]
    fn parse_request_with_whitespace() {
        let input = "<request-tool>\n  <tool> Bash </tool>\n  <reason> run tests </reason>\n</request-tool>";
        let reqs = parse_requests(input);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "Bash");
        assert_eq!(reqs[0].reason, "run tests");
    }

    #[test]
    fn prompt_approve_then_deny() {
        let reqs = vec![
            ToolRequest {
                tool: "Bash".into(),
                reason: "a".into(),
            },
            ToolRequest {
                tool: "WebFetch".into(),
                reason: "b".into(),
            },
        ];
        let mut input = b"y\nn\n" as &[u8];
        let mut output = Vec::new();
        let reviewed = prompt_requests(&reqs, &mut input, &mut output).unwrap();
        assert_eq!(reviewed.len(), 2);
        assert_eq!(reviewed[0].decision, Decision::Granted);
        assert_eq!(reviewed[1].decision, Decision::Denied);
    }

    #[test]
    fn summary_mixed() {
        let reviewed = vec![
            ReviewedRequest {
                request: ToolRequest {
                    tool: "Bash".into(),
                    reason: "a".into(),
                },
                decision: Decision::Granted,
            },
            ReviewedRequest {
                request: ToolRequest {
                    tool: "WebFetch".into(),
                    reason: "b".into(),
                },
                decision: Decision::Denied,
            },
        ];
        let summary = format_summary(&reviewed);
        assert!(summary.contains("Granted: Bash"));
        assert!(summary.contains("Denied: WebFetch"));
    }

    #[test]
    fn summary_all_granted() {
        let reviewed = vec![ReviewedRequest {
            request: ToolRequest {
                tool: "Bash".into(),
                reason: "a".into(),
            },
            decision: Decision::Granted,
        }];
        let summary = format_summary(&reviewed);
        assert_eq!(summary, "Granted: Bash");
    }

    #[test]
    fn summary_empty() {
        let summary = format_summary(&[]);
        assert_eq!(summary, "No tool requests.");
    }
}
