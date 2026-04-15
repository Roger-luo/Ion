/// A parsed `<request-tool>` message from an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequest {
    pub tool: String,
    pub reason: String,
    pub scope: Option<String>,
}

/// Errors that can occur when parsing a `<request-tool>` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The `<request-tool>` wrapper element is missing.
    MissingRequestTool,
    /// The required `<tool>` element is missing or empty.
    MissingTool,
    /// The required `<reason>` element is missing or empty.
    MissingReason,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingRequestTool => {
                write!(f, "missing <request-tool> element")
            }
            ParseError::MissingTool => {
                write!(f, "missing or empty <tool> in <request-tool>")
            }
            ParseError::MissingReason => {
                write!(f, "missing or empty <reason> in <request-tool>")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Extract text content between `<tag>` and `</tag>`, returning `None`
/// when the tag is absent or its trimmed content is empty.
fn extract_element(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    let text = xml[start..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Parse a `<request-tool>` XML fragment emitted by a research agent.
///
/// Both `<tool>` and `<reason>` are required and must be non-empty.
/// `<scope>` is optional.
///
/// # Errors
///
/// Returns [`ParseError`] when the input is malformed or a required
/// element is missing.
pub fn parse_tool_request(xml: &str) -> Result<ToolRequest, ParseError> {
    // Ensure the wrapper element is present.
    if !xml.contains("<request-tool>") {
        return Err(ParseError::MissingRequestTool);
    }

    let tool = extract_element(xml, "tool").ok_or(ParseError::MissingTool)?;
    let reason = extract_element(xml, "reason").ok_or(ParseError::MissingReason)?;
    let scope = extract_element(xml, "scope");

    Ok(ToolRequest {
        tool,
        reason,
        scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_request() {
        let xml =
            "<request-tool><tool>Bash</tool><reason>Need to run tests</reason></request-tool>";
        let req = parse_tool_request(xml).unwrap();
        assert_eq!(req.tool, "Bash");
        assert_eq!(req.reason, "Need to run tests");
        assert_eq!(req.scope, None);
    }

    #[test]
    fn parse_valid_request_with_scope() {
        let xml = "<request-tool><tool>Bash</tool><reason>Check deps</reason><scope>cargo tree:*</scope></request-tool>";
        let req = parse_tool_request(xml).unwrap();
        assert_eq!(req.tool, "Bash");
        assert_eq!(req.reason, "Check deps");
        assert_eq!(req.scope.as_deref(), Some("cargo tree:*"));
    }

    #[test]
    fn missing_reason_returns_error() {
        let xml = "<request-tool><tool>Bash</tool></request-tool>";
        let err = parse_tool_request(xml).unwrap_err();
        assert_eq!(err, ParseError::MissingReason);
        assert!(
            err.to_string().contains("reason"),
            "error message should mention reason: {err}"
        );
    }

    #[test]
    fn empty_reason_returns_error() {
        let xml = "<request-tool><tool>Bash</tool><reason></reason></request-tool>";
        let err = parse_tool_request(xml).unwrap_err();
        assert_eq!(err, ParseError::MissingReason);
    }

    #[test]
    fn whitespace_only_reason_returns_error() {
        let xml = "<request-tool><tool>Bash</tool><reason>   </reason></request-tool>";
        let err = parse_tool_request(xml).unwrap_err();
        assert_eq!(err, ParseError::MissingReason);
    }

    #[test]
    fn missing_tool_returns_error() {
        let xml = "<request-tool><reason>Need it</reason></request-tool>";
        let err = parse_tool_request(xml).unwrap_err();
        assert_eq!(err, ParseError::MissingTool);
    }

    #[test]
    fn missing_wrapper_returns_error() {
        let xml = "<tool>Bash</tool><reason>Need it</reason>";
        let err = parse_tool_request(xml).unwrap_err();
        assert_eq!(err, ParseError::MissingRequestTool);
    }

    #[test]
    fn no_panic_on_empty_input() {
        let err = parse_tool_request("").unwrap_err();
        assert_eq!(err, ParseError::MissingRequestTool);
    }

    #[test]
    fn no_panic_on_garbage_input() {
        let err = parse_tool_request("not xml at all").unwrap_err();
        assert_eq!(err, ParseError::MissingRequestTool);
    }
}
