use serde::{Deserialize, Serialize};

/// Configuration for AGENTS.md template management.
/// Parsed from [agents] in Ion.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsConfig {
    /// Template source (GitHub shorthand, Git URL, HTTP, or local path)
    #[serde(default)]
    pub template: Option<String>,
    /// Pin to a specific git revision
    #[serde(default)]
    pub rev: Option<String>,
    /// Path to AGENTS.md within the source repo (default: "AGENTS.md" at root)
    #[serde(default)]
    pub path: Option<String>,
}

/// Lock entry for the AGENTS.md template.
/// Tracks the last-synced state in Ion.lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsLockEntry {
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub checksum: String,
    pub updated_at: String, // ISO 8601, stored as plain string
}
