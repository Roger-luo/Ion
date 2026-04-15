//! Autotune — lightweight `<request-tool>` handling for agent interactions.
//!
//! Parses `<request-tool>` XML fragments from agent response text and tracks
//! which tools the user has already approved so that duplicate requests are
//! silently skipped.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::sync::OnceLock;

use regex::Regex;

fn tool_request_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"<request-tool>\s*(\S+)\s*</request-tool>")
            .expect("request-tool regex must compile")
    })
}

/// Extract tool names from `<request-tool>TOOL</request-tool>` fragments.
pub fn parse_tool_requests(text: &str) -> Vec<String> {
    tool_request_regex()
        .captures_iter(text)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Tracks which tools the user has approved during a session.
#[derive(Debug, Default)]
pub struct ToolSession {
    approved: HashSet<String>,
}

impl ToolSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the tool has already been approved.
    pub fn is_approved(&self, tool: &str) -> bool {
        self.approved.contains(tool)
    }

    /// Mark a tool as approved.
    pub fn approve(&mut self, tool: &str) {
        self.approved.insert(tool.to_string());
    }

    /// Process a chunk of agent output, prompting for unapproved tools.
    ///
    /// For each `<request-tool>` found:
    /// - already approved → silently skipped
    /// - new tool → prompt on `writer`, read decision from `reader`
    ///
    /// Returns the list of tool names that were newly approved.
    pub fn process_response<R: BufRead, W: Write>(
        &mut self,
        text: &str,
        reader: &mut R,
        writer: &mut W,
    ) -> std::io::Result<Vec<String>> {
        let requests = parse_tool_requests(text);
        let mut newly_approved = Vec::new();

        for tool in requests {
            if self.is_approved(&tool) {
                continue;
            }

            write!(writer, "Allow tool '{tool}'? [y/n]: ")?;
            writer.flush()?;

            let mut answer = String::new();
            reader.read_line(&mut answer)?;

            if answer.trim().eq_ignore_ascii_case("y") {
                self.approve(&tool);
                newly_approved.push(tool);
            }
        }

        Ok(newly_approved)
    }

    /// Return a snapshot of the currently approved tools.
    pub fn approved_tools(&self) -> &HashSet<String> {
        &self.approved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_single_request_tool() {
        let text = "Hello <request-tool>Bash</request-tool> world";
        let tools = parse_tool_requests(text);
        assert_eq!(tools, vec!["Bash"]);
    }

    #[test]
    fn parse_multiple_request_tools() {
        let text = "<request-tool>Bash</request-tool> text <request-tool>Read</request-tool>";
        let tools = parse_tool_requests(text);
        assert_eq!(tools, vec!["Bash", "Read"]);
    }

    #[test]
    fn parse_no_request_tools() {
        let text = "No tools here";
        let tools = parse_tool_requests(text);
        assert!(tools.is_empty());
    }

    #[test]
    fn parse_whitespace_around_tool_name() {
        let text = "<request-tool> Bash </request-tool>";
        let tools = parse_tool_requests(text);
        assert_eq!(tools, vec!["Bash"]);
    }

    #[test]
    fn session_approves_tool() {
        let mut session = ToolSession::new();
        assert!(!session.is_approved("Bash"));
        session.approve("Bash");
        assert!(session.is_approved("Bash"));
    }

    #[test]
    fn process_response_prompts_for_new_tool() {
        let mut session = ToolSession::new();
        let text = "<request-tool>Bash</request-tool>";
        let mut input = Cursor::new(b"y\n");
        let mut output = Vec::new();

        let approved = session
            .process_response(text, &mut input, &mut output)
            .unwrap();
        assert_eq!(approved, vec!["Bash"]);
        assert!(session.is_approved("Bash"));
        let prompt = String::from_utf8(output).unwrap();
        assert!(prompt.contains("Allow tool 'Bash'?"));
    }

    #[test]
    fn process_response_skips_already_approved() {
        let mut session = ToolSession::new();
        session.approve("Bash");

        let text = "<request-tool>Bash</request-tool>";
        let mut input = Cursor::new(b"");
        let mut output = Vec::new();

        let approved = session
            .process_response(text, &mut input, &mut output)
            .unwrap();
        assert!(approved.is_empty());
        // No prompt should have been written
        assert!(output.is_empty());
    }

    #[test]
    fn process_response_denied_tool_not_approved() {
        let mut session = ToolSession::new();
        let text = "<request-tool>Bash</request-tool>";
        let mut input = Cursor::new(b"n\n");
        let mut output = Vec::new();

        let approved = session
            .process_response(text, &mut input, &mut output)
            .unwrap();
        assert!(approved.is_empty());
        assert!(!session.is_approved("Bash"));
    }

    #[test]
    fn duplicate_request_in_same_response_only_prompts_once() {
        let mut session = ToolSession::new();
        let text = "<request-tool>Bash</request-tool> text <request-tool>Bash</request-tool>";
        let mut input = Cursor::new(b"y\n");
        let mut output = Vec::new();

        let approved = session
            .process_response(text, &mut input, &mut output)
            .unwrap();
        assert_eq!(approved, vec!["Bash"]);
        // After approving, second occurrence is skipped — no second prompt
        let prompt = String::from_utf8(output).unwrap();
        let count = prompt.matches("Allow tool 'Bash'?").count();
        assert_eq!(count, 1);
    }
}
