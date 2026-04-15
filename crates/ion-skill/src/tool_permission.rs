use std::fmt;

/// A permission granted to a session for a specific tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPermission {
    /// Unrestricted access to the named tool.
    Allow(String),
    /// Access to the named tool, restricted to commands matching the scope pattern.
    AllowScoped { tool: String, scope: String },
}

/// A tool request parsed from a `<request-tool>` element in a skill body.
///
/// A request may optionally include a `<scope>` to narrow the permission.
/// For example:
///
/// ```xml
/// <request-tool>
///   <tool>Bash</tool>
///   <scope>cargo tree:*</scope>
/// </request-tool>
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequest {
    /// The tool name, e.g. `"Bash"`.
    pub tool: String,
    /// An optional scope pattern that narrows the request, e.g. `"cargo tree:*"`.
    pub scope: Option<String>,
}

impl ToolRequest {
    /// Create a new unscoped tool request.
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            scope: None,
        }
    }

    /// Create a new scoped tool request.
    pub fn scoped(tool: impl Into<String>, scope: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            scope: Some(scope.into()),
        }
    }

    /// The label shown in the approval prompt.
    ///
    /// Scoped requests display as `Tool(scope)` (e.g. `Bash(cargo tree:*)`);
    /// unscoped requests display the bare tool name (e.g. `Bash`).
    pub fn approval_label(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}({})", self.tool, scope),
            None => self.tool.clone(),
        }
    }

    /// Produce the [`ToolPermission`] that should be recorded in the session
    /// when the user approves this request.
    pub fn grant(&self) -> ToolPermission {
        match &self.scope {
            Some(scope) => ToolPermission::AllowScoped {
                tool: self.tool.clone(),
                scope: scope.clone(),
            },
            None => ToolPermission::Allow(self.tool.clone()),
        }
    }
}

impl fmt::Display for ToolRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.approval_label())
    }
}

impl fmt::Display for ToolPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolPermission::Allow(tool) => write!(f, "Allow({})", tool),
            ToolPermission::AllowScoped { tool, scope } => {
                write!(f, "AllowScoped({}, {})", tool, scope)
            }
        }
    }
}

/// Parse all `<request-tool>` elements from a skill body.
///
/// Each element must contain a `<tool>` child; an optional `<scope>` child
/// narrows the request.  Returns an empty vec when no elements are found.
pub fn parse_tool_requests(body: &str) -> Vec<ToolRequest> {
    let mut requests = Vec::new();
    let mut search_from = 0;

    while let Some(start) = body[search_from..].find("<request-tool>") {
        let abs_start = search_from + start;
        let after_tag = abs_start + "<request-tool>".len();

        let Some(end) = body[after_tag..].find("</request-tool>") else {
            break;
        };
        let inner = &body[after_tag..after_tag + end];

        if let Some(req) = parse_single_request(inner) {
            requests.push(req);
        }

        search_from = after_tag + end + "</request-tool>".len();
    }

    requests
}

/// Parse a single `<request-tool>` inner content.
fn parse_single_request(inner: &str) -> Option<ToolRequest> {
    let tool = extract_tag(inner, "tool")?;
    let scope = extract_tag(inner, "scope");
    Some(ToolRequest { tool, scope })
}

/// Extract the text content of the first occurrence of `<tag>…</tag>`.
fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)?;
    let content = text[start..start + end].trim();
    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ToolRequest construction & label ───────────────────────────

    #[test]
    fn unscoped_request_label_is_bare_tool_name() {
        let req = ToolRequest::new("Bash");
        assert_eq!(req.approval_label(), "Bash");
    }

    #[test]
    fn scoped_request_label_includes_scope() {
        let req = ToolRequest::scoped("Bash", "cargo tree:*");
        assert_eq!(req.approval_label(), "Bash(cargo tree:*)");
    }

    #[test]
    fn display_matches_approval_label() {
        let req = ToolRequest::scoped("Bash", "cargo tree:*");
        assert_eq!(req.to_string(), "Bash(cargo tree:*)");
    }

    // ── grant() produces correct ToolPermission ───────────────────

    #[test]
    fn unscoped_request_grants_allow() {
        let req = ToolRequest::new("Read");
        assert_eq!(req.grant(), ToolPermission::Allow("Read".into()));
    }

    #[test]
    fn scoped_request_grants_allow_scoped() {
        let req = ToolRequest::scoped("Bash", "cargo tree:*");
        assert_eq!(
            req.grant(),
            ToolPermission::AllowScoped {
                tool: "Bash".into(),
                scope: "cargo tree:*".into(),
            }
        );
    }

    #[test]
    fn scoped_grant_is_not_unscoped_allow() {
        let req = ToolRequest::scoped("Bash", "cargo tree:*");
        let perm = req.grant();
        // Must NOT be the unscoped variant
        assert!(
            !matches!(perm, ToolPermission::Allow(_)),
            "scoped request must produce AllowScoped, not Allow"
        );
    }

    // ── parse_tool_requests ───────────────────────────────────────

    #[test]
    fn parse_scoped_request() {
        let body = r#"
Some instructions.

<request-tool>
  <tool>Bash</tool>
  <scope>cargo tree:*</scope>
</request-tool>

More text.
"#;
        let requests = parse_tool_requests(body);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].tool, "Bash");
        assert_eq!(requests[0].scope.as_deref(), Some("cargo tree:*"));
    }

    #[test]
    fn parse_unscoped_request() {
        let body = r#"
<request-tool>
  <tool>Read</tool>
</request-tool>
"#;
        let requests = parse_tool_requests(body);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].tool, "Read");
        assert_eq!(requests[0].scope, None);
    }

    #[test]
    fn parse_multiple_requests() {
        let body = r#"
<request-tool>
  <tool>Bash</tool>
  <scope>cargo tree:*</scope>
</request-tool>
<request-tool>
  <tool>Read</tool>
</request-tool>
<request-tool>
  <tool>Bash</tool>
  <scope>npm run build</scope>
</request-tool>
"#;
        let requests = parse_tool_requests(body);
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].approval_label(), "Bash(cargo tree:*)");
        assert_eq!(requests[1].approval_label(), "Read");
        assert_eq!(requests[2].approval_label(), "Bash(npm run build)");
    }

    #[test]
    fn parse_no_requests() {
        let requests = parse_tool_requests("Just some text with no tool requests.");
        assert!(requests.is_empty());
    }

    #[test]
    fn parse_ignores_malformed_request_without_tool() {
        let body = r#"
<request-tool>
  <scope>some scope</scope>
</request-tool>
"#;
        let requests = parse_tool_requests(body);
        assert!(requests.is_empty());
    }

    // ── Scenario: scoped request-tool ─────────────────────────────
    // The scenario the issue asks for: a <request-tool> with a <scope>.
    // Verify:
    //   1. The approval prompt shows the scoped label, not just the bare name.
    //   2. On approval, the permission is AllowScoped, not unscoped Allow.

    #[test]
    fn scenario_scoped_request_tool_approval_label() {
        // Parse a skill body that contains a scoped <request-tool>.
        let body = r#"
This skill needs to inspect the dependency tree.

<request-tool>
  <tool>Bash</tool>
  <scope>cargo tree:*</scope>
</request-tool>

Use the output to advise on dependency management.
"#;
        let requests = parse_tool_requests(body);
        assert_eq!(requests.len(), 1, "should parse exactly one request");

        let req = &requests[0];

        // 1. The approval prompt must show the scoped label.
        assert_eq!(
            req.approval_label(),
            "Bash(cargo tree:*)",
            "approval prompt must show Bash(cargo tree:*), not bare Bash"
        );
        assert_ne!(
            req.approval_label(),
            "Bash",
            "approval label must not be the bare tool name when a scope is present"
        );
    }

    #[test]
    fn scenario_scoped_request_tool_grants_allow_scoped() {
        let body = r#"
<request-tool>
  <tool>Bash</tool>
  <scope>cargo tree:*</scope>
</request-tool>
"#;
        let requests = parse_tool_requests(body);
        let req = &requests[0];

        // 2. The granted permission must be AllowScoped, not Allow.
        let perm = req.grant();
        assert_eq!(
            perm,
            ToolPermission::AllowScoped {
                tool: "Bash".into(),
                scope: "cargo tree:*".into(),
            },
            "granting a scoped request must produce AllowScoped"
        );
        assert!(
            !matches!(perm, ToolPermission::Allow(_)),
            "granting a scoped request must NOT produce unscoped Allow"
        );
    }
}
