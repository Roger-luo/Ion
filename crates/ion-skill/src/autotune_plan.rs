//! Parsing and dispatch logic for agent responses that may contain
//! `<plan>` and/or `<request-tool>` fragments.
//!
//! ## Behavioral contract (Option A)
//!
//! When a response contains **both** `<request-tool>` and `<plan>` fragments,
//! tool requests are handled first and the plan is **ignored**.
//! Rationale: tool grants change the available context, so any plan emitted
//! alongside a tool request is stale by definition.
//!
//! This contract is enforced by [`plan_next`] returning `None` whenever
//! the response also contains tool requests. See the tests at the bottom
//! of this module for the pinned scenario.

use regex::Regex;
use std::sync::LazyLock;

// ── data types ──────────────────────────────────────────────────────

/// A parsed `<request-tool>` fragment from an agent response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequest {
    /// Name of the requested tool (e.g. `"Bash"`, `"WebFetch"`).
    pub tool: String,
    /// Why the agent wants the tool.
    pub reason: String,
}

/// A parsed `<plan>` fragment from an agent response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The plan/hypothesis content.
    pub content: String,
}

// ── regex patterns ──────────────────────────────────────────────────

static REQUEST_TOOL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)<request-tool>\s*<tool>\s*(.*?)\s*</tool>\s*<reason>\s*(.*?)\s*</reason>\s*</request-tool>",
    )
    .expect("request-tool regex must compile")
});

static PLAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<plan>\s*(.*?)\s*</plan>").expect("plan regex must compile"));

// ── public API ──────────────────────────────────────────────────────

/// Extract all `<request-tool>` fragments from an agent response.
///
/// Returns an empty `Vec` when the response contains no tool requests.
pub fn handle_tool_requests(response: &str) -> Vec<ToolRequest> {
    REQUEST_TOOL_RE
        .captures_iter(response)
        .map(|cap| ToolRequest {
            tool: cap[1].to_string(),
            reason: cap[2].to_string(),
        })
        .collect()
}

/// Extract the `<plan>` from an agent response, applying the
/// **Option A** rule: if the response also contains `<request-tool>`
/// fragments the plan is discarded and `None` is returned.
pub fn plan_next(response: &str) -> Option<Plan> {
    // Option A: tool requests take priority over plans.
    if REQUEST_TOOL_RE.is_match(response) {
        return None;
    }

    PLAN_RE.captures(response).map(|cap| Plan {
        content: cap[1].to_string(),
    })
}

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── handle_tool_requests ────────────────────────────────────────

    #[test]
    fn no_tool_requests() {
        let resp = "Just some plain text, nothing special.";
        assert!(handle_tool_requests(resp).is_empty());
    }

    #[test]
    fn single_tool_request() {
        let resp = "\
            <request-tool>\
              <tool>Bash</tool>\
              <reason>Need to check dependencies</reason>\
            </request-tool>";
        let reqs = handle_tool_requests(resp);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "Bash");
        assert_eq!(reqs[0].reason, "Need to check dependencies");
    }

    #[test]
    fn multiple_tool_requests() {
        let resp = "\
            <request-tool><tool>Bash</tool><reason>a</reason></request-tool>\
            <request-tool><tool>WebFetch</tool><reason>b</reason></request-tool>";
        let reqs = handle_tool_requests(resp);
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].tool, "Bash");
        assert_eq!(reqs[1].tool, "WebFetch");
    }

    // ── plan_next ───────────────────────────────────────────────────

    #[test]
    fn plan_only() {
        let resp = "<plan>1. Check deps\n2. Run tests</plan>";
        let plan = plan_next(resp).expect("should parse plan");
        assert_eq!(plan.content, "1. Check deps\n2. Run tests");
    }

    #[test]
    fn no_plan() {
        let resp = "Just some text without any plan.";
        assert!(plan_next(resp).is_none());
    }

    // ── mixed response: pinned Option A behavior ────────────────────

    /// **Contract test (Issue #138, Option A):**
    /// When the agent disobeys the prompt and emits both a `<plan>` and
    /// a `<request-tool>` in the same turn, tool requests are extracted
    /// normally but `plan_next` returns `None` — the plan is discarded.
    #[test]
    fn mixed_plan_and_tool_request_discards_plan() {
        let resp = "\
            <plan>1. Explore the repo\n2. Write a fix</plan>\n\
            <request-tool><tool>Bash</tool><reason>need shell access</reason></request-tool>";

        // Tool requests are still extracted.
        let reqs = handle_tool_requests(resp);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "Bash");

        // Plan is discarded (Option A).
        assert!(
            plan_next(resp).is_none(),
            "plan_next must return None when the response also contains tool requests \
             (Option A: tool requests take priority)"
        );
    }

    /// Same contract, but with tool request appearing *before* the plan.
    #[test]
    fn mixed_tool_request_before_plan_discards_plan() {
        let resp = "\
            <request-tool><tool>WebFetch</tool><reason>fetch docs</reason></request-tool>\n\
            <plan>Research the API</plan>";

        let reqs = handle_tool_requests(resp);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "WebFetch");

        assert!(
            plan_next(resp).is_none(),
            "plan_next must return None regardless of fragment ordering"
        );
    }

    /// Multiple tool requests alongside a plan: all requests extracted, plan ignored.
    #[test]
    fn mixed_multiple_tool_requests_and_plan_discards_plan() {
        let resp = "\
            <request-tool><tool>Bash</tool><reason>a</reason></request-tool>\
            <plan>Do stuff</plan>\
            <request-tool><tool>WebFetch</tool><reason>b</reason></request-tool>";

        let reqs = handle_tool_requests(resp);
        assert_eq!(reqs.len(), 2);

        assert!(
            plan_next(resp).is_none(),
            "plan_next must return None even with multiple tool requests"
        );
    }
}
