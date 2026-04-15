/// A permission granted to an agent for a specific tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolPermission {
    /// Allow the named tool globally (e.g. `Allow("Bash")`).
    Allow(String),
    /// Allow the named tool with a scope constraint (e.g. `AllowScoped("Bash", "cargo test:*")`).
    AllowScoped(String, String),
}

impl ToolPermission {
    /// Returns the tool name regardless of variant.
    pub fn tool_name(&self) -> &str {
        match self {
            ToolPermission::Allow(name) | ToolPermission::AllowScoped(name, _) => name,
        }
    }
}

/// Configuration passed to an agent on each spawn or send call.
#[derive(Debug, Clone, Default)]
pub struct AgentConfig {
    /// Tools the agent is currently allowed to use.
    pub allowed_tools: Vec<ToolPermission>,
}

/// Trait representing an AI agent backend.
///
/// Implementations receive an `AgentConfig` on every call so that newly
/// approved permissions are visible to the agent immediately.
pub trait Agent {
    /// Start the agent with the given configuration.
    fn spawn(&self, config: &AgentConfig) -> crate::Result<String>;

    /// Send a follow-up message to a running agent.
    fn send(&self, message: &str, config: &AgentConfig) -> crate::Result<String>;
}

/// Manages a session's approved tool permissions and wires them into
/// every agent call.
pub struct ToolSession<A: Agent> {
    agent: A,
    approved: Vec<ToolPermission>,
}

impl<A: Agent> ToolSession<A> {
    /// Create a new session with no approved tools.
    pub fn new(agent: A) -> Self {
        Self {
            agent,
            approved: Vec::new(),
        }
    }

    /// Grant a tool permission for the remainder of this session.
    pub fn grant(&mut self, permission: ToolPermission) {
        if !self.approved.contains(&permission) {
            self.approved.push(permission);
        }
    }

    /// Build the current `AgentConfig` snapshot.
    fn config(&self) -> AgentConfig {
        AgentConfig {
            allowed_tools: self.approved.clone(),
        }
    }

    /// Spawn the agent, forwarding current permissions.
    pub fn spawn(&self) -> crate::Result<String> {
        self.agent.spawn(&self.config())
    }

    /// Send a message, forwarding current permissions.
    pub fn send(&self, message: &str) -> crate::Result<String> {
        self.agent.send(message, &self.config())
    }
}

/// A test double that records the `AgentConfig` received on every call.
#[cfg(test)]
pub struct MockAgent {
    /// Scripted responses returned by `spawn` and `send` in order.
    responses: std::sync::Mutex<Vec<String>>,
    /// Records the `allowed_tools` seen on each call (spawn or send).
    pub allowed_tools_history: std::sync::Mutex<Vec<Vec<ToolPermission>>>,
}

#[cfg(test)]
impl MockAgent {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses),
            allowed_tools_history: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn record_and_respond(&self, config: &AgentConfig) -> crate::Result<String> {
        self.allowed_tools_history
            .lock()
            .unwrap()
            .push(config.allowed_tools.clone());

        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Err(crate::Error::Other(
                "MockAgent: no more scripted responses".into(),
            ));
        }
        Ok(responses.remove(0))
    }
}

#[cfg(test)]
impl Agent for MockAgent {
    fn spawn(&self, config: &AgentConfig) -> crate::Result<String> {
        self.record_and_respond(config)
    }

    fn send(&self, _message: &str, config: &AgentConfig) -> crate::Result<String> {
        self.record_and_respond(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_tool_passed_on_next_send() {
        // Set up a mock agent with two scripted responses:
        // 1. First spawn returns a tool-request (simulating <request-tool>Bash</request-tool>)
        // 2. Second send returns a plan (simulating <plan>...</plan>)
        let agent = MockAgent::new(vec![
            "<request-tool>Bash</request-tool>".to_string(),
            "<plan>run tests</plan>".to_string(),
        ]);

        let mut session = ToolSession::new(agent);

        // --- Call 1: spawn (before any approval) ---
        let response = session.spawn().unwrap();
        assert_eq!(response, "<request-tool>Bash</request-tool>");

        // Verify: Bash does NOT appear in allowed_tools before approval
        {
            let history = session.agent.allowed_tools_history.lock().unwrap();
            assert_eq!(history.len(), 1);
            assert!(
                history[0].is_empty(),
                "No tools should be allowed before approval"
            );
        }

        // --- Simulate PTY approval: user says "y" → grant Bash ---
        session.grant(ToolPermission::Allow("Bash".to_string()));

        // --- Call 2: send (after approval) ---
        let response = session.send("continue").unwrap();
        assert_eq!(response, "<plan>run tests</plan>");

        // Verify: Bash NOW appears in allowed_tools for the follow-up call
        {
            let history = session.agent.allowed_tools_history.lock().unwrap();
            assert_eq!(history.len(), 2);

            // First call had no permissions
            assert!(
                history[0].is_empty(),
                "First call should have no allowed tools"
            );

            // Second call carries the approved Bash permission
            assert_eq!(history[1].len(), 1);
            assert_eq!(
                history[1][0],
                ToolPermission::Allow("Bash".to_string()),
                "Bash must appear in allowed_tools after approval"
            );
        }
    }

    #[test]
    fn approved_tool_persists_across_multiple_sends() {
        let agent = MockAgent::new(vec![
            "response-1".to_string(),
            "response-2".to_string(),
            "response-3".to_string(),
        ]);

        let mut session = ToolSession::new(agent);

        // Call 1: no permissions yet
        session.spawn().unwrap();

        // Approve Bash
        session.grant(ToolPermission::Allow("Bash".to_string()));

        // Call 2: after approval
        session.send("msg-2").unwrap();

        // Call 3: Bash should still be present
        session.send("msg-3").unwrap();

        let history = session.agent.allowed_tools_history.lock().unwrap();
        assert_eq!(history.len(), 3);

        // Call 1: empty
        assert!(history[0].is_empty());

        // Calls 2 and 3: Bash present
        for call in &history[1..] {
            let bash_perm = ToolPermission::Allow("Bash".to_string());
            assert!(
                call.contains(&bash_perm),
                "Bash must appear in allowed_tools for all sends after approval"
            );
        }
    }

    #[test]
    fn scoped_permission_passed_correctly() {
        let agent = MockAgent::new(vec!["resp-1".to_string(), "resp-2".to_string()]);

        let mut session = ToolSession::new(agent);

        session.spawn().unwrap();

        // Approve Bash with a scope
        session.grant(ToolPermission::AllowScoped(
            "Bash".to_string(),
            "cargo test:*".to_string(),
        ));

        session.send("next").unwrap();

        let history = session.agent.allowed_tools_history.lock().unwrap();

        // First call: no permissions
        assert!(history[0].is_empty());

        // Second call: scoped permission present
        assert_eq!(
            history[1],
            vec![ToolPermission::AllowScoped(
                "Bash".to_string(),
                "cargo test:*".to_string()
            )]
        );
    }

    #[test]
    fn duplicate_grant_is_deduplicated() {
        let agent = MockAgent::new(vec!["resp".to_string()]);

        let mut session = ToolSession::new(agent);

        session.grant(ToolPermission::Allow("Bash".to_string()));
        session.grant(ToolPermission::Allow("Bash".to_string()));

        session.spawn().unwrap();

        let history = session.agent.allowed_tools_history.lock().unwrap();
        assert_eq!(
            history[0].len(),
            1,
            "Duplicate grants should be deduplicated"
        );
    }

    #[test]
    fn tool_name_returns_correct_name() {
        let global = ToolPermission::Allow("Read".to_string());
        assert_eq!(global.tool_name(), "Read");

        let scoped = ToolPermission::AllowScoped("Bash".to_string(), "ls:*".to_string());
        assert_eq!(scoped.tool_name(), "Bash");
    }
}
