# Search Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ion search <query>` to discover skills across skills.sh, custom registries, GitHub, and an optional CLI agent.

**Architecture:** A `SearchSource` trait with four implementations (SkillsSh, CustomRegistry, GitHub, Agent) lives in `ion-skill`. The CLI command in `src/commands/search.rs` orchestrates cascade vs parallel search. We use `reqwest::blocking` for HTTP and `std::thread` for parallel mode to avoid converting the existing sync codebase to async.

**Tech Stack:** reqwest (blocking, rustls-tls), dialoguer, std::thread

---

### Task 1: Add reqwest and dialoguer dependencies

**Files:**
- Modify: `crates/ion-skill/Cargo.toml`
- Modify: `Cargo.toml`

**Step 1: Add reqwest to ion-skill**

In `crates/ion-skill/Cargo.toml`, add to `[dependencies]`:

```toml
reqwest = { version = "0.12", features = ["blocking", "rustls-tls"], default-features = false }
```

**Step 2: Add dialoguer to root crate**

In `Cargo.toml`, add to `[dependencies]`:

```toml
dialoguer = "0.11"
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles without errors

**Step 4: Commit**

```bash
git add Cargo.toml crates/ion-skill/Cargo.toml Cargo.lock
git commit -m "deps: add reqwest and dialoguer for search command"
```

---

### Task 2: Add SearchResult and SearchSource trait

**Files:**
- Create: `crates/ion-skill/src/search.rs`
- Modify: `crates/ion-skill/src/lib.rs`

**Step 1: Write the test**

Create `crates/ion-skill/src/search.rs` with:

```rust
/// Search result from any source.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub source: String,
    pub registry: String,
}

/// A searchable source of skills.
pub trait SearchSource {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSource {
        results: Vec<SearchResult>,
    }

    impl SearchSource for FakeSource {
        fn name(&self) -> &str {
            "fake"
        }
        fn search(&self, _query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
            Ok(self.results.iter().take(limit).cloned().collect())
        }
    }

    #[test]
    fn trait_search_returns_results() {
        let source = FakeSource {
            results: vec![
                SearchResult {
                    name: "test-skill".to_string(),
                    description: "A test".to_string(),
                    source: "owner/repo/test-skill".to_string(),
                    registry: "fake".to_string(),
                },
            ],
        };
        let results = source.search("test", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }

    #[test]
    fn trait_search_respects_limit() {
        let source = FakeSource {
            results: vec![
                SearchResult {
                    name: "a".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
                SearchResult {
                    name: "b".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
                SearchResult {
                    name: "c".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
            ],
        };
        let results = source.search("x", 2).unwrap();
        assert_eq!(results.len(), 2);
    }
}
```

**Step 2: Register the module**

In `crates/ion-skill/src/lib.rs`, add:

```rust
pub mod search;
```

**Step 3: Run tests**

Run: `cargo test -p ion-skill search`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add crates/ion-skill/src/search.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add SearchResult and SearchSource trait"
```

---

### Task 3: Add Error::Search variant

**Files:**
- Modify: `crates/ion-skill/src/error.rs`

**Step 1: Add the variant**

Add to the `Error` enum in `crates/ion-skill/src/error.rs`:

```rust
    #[error("Search error: {0}")]
    Search(String),

    #[error("HTTP error: {0}")]
    Http(String),
```

**Step 2: Verify it compiles**

Run: `cargo check -p ion-skill`
Expected: compiles

**Step 3: Commit**

```bash
git add crates/ion-skill/src/error.rs
git commit -m "feat: add Search and Http error variants"
```

---

### Task 4: Implement RegistrySource (covers skills.sh and custom registries)

**Files:**
- Modify: `crates/ion-skill/src/search.rs`

The skills.sh API and custom registries share the same contract. One struct handles both — just different base URLs.

**Step 1: Write the test**

Add to the `tests` module in `search.rs`:

```rust
    #[test]
    fn registry_source_parses_json_response() {
        // Test the parsing logic directly
        let json = r#"[
            {"name": "brainstorming", "description": "Brainstorm ideas", "source": "anthropics/skills/brainstorming"},
            {"name": "tdd", "description": "Test driven dev", "source": "anthropics/skills/tdd"}
        ]"#;
        let results = parse_registry_response(json, "skills.sh", 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "brainstorming");
        assert_eq!(results[0].registry, "skills.sh");
        assert_eq!(results[1].source, "anthropics/skills/tdd");
    }

    #[test]
    fn registry_source_respects_limit() {
        let json = r#"[
            {"name": "a", "description": "d1", "source": "s1"},
            {"name": "b", "description": "d2", "source": "s2"},
            {"name": "c", "description": "d3", "source": "s3"}
        ]"#;
        let results = parse_registry_response(json, "test", 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn registry_source_handles_empty_response() {
        let json = "[]";
        let results = parse_registry_response(json, "test", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn registry_source_handles_malformed_json() {
        let json = "not json";
        let result = parse_registry_response(json, "test", 10);
        assert!(result.is_err());
    }
```

**Step 2: Run tests, verify they fail**

Run: `cargo test -p ion-skill search`
Expected: FAIL — `parse_registry_response` not found

**Step 3: Implement RegistrySource**

Add above the `tests` module in `search.rs`:

```rust
use serde::Deserialize;

/// A JSON entry from a registry API response.
#[derive(Deserialize)]
struct RegistryEntry {
    name: String,
    description: String,
    source: String,
}

/// Parse a JSON array of registry entries into SearchResults.
pub fn parse_registry_response(
    body: &str,
    registry_name: &str,
    limit: usize,
) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<RegistryEntry> =
        serde_json::from_str(body).map_err(|e| crate::Error::Search(format!("Invalid registry response: {e}")))?;
    Ok(entries
        .into_iter()
        .take(limit)
        .map(|e| SearchResult {
            name: e.name,
            description: e.description,
            source: e.source,
            registry: registry_name.to_string(),
        })
        .collect())
}

/// Searches a registry (skills.sh or custom) via HTTP GET.
pub struct RegistrySource {
    pub registry_name: String,
    pub base_url: String,
}

impl SearchSource for RegistrySource {
    fn name(&self) -> &str {
        &self.registry_name
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        let url = format!("{}/search?q={}&limit={}", self.base_url.trim_end_matches('/'), query, limit);
        let response = reqwest::blocking::get(&url)
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?;
        let body = response
            .text()
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?;
        parse_registry_response(&body, &self.registry_name, limit)
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill search`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/ion-skill/src/search.rs
git commit -m "feat: implement RegistrySource for skills.sh and custom registries"
```

---

### Task 5: Implement GitHubSource

**Files:**
- Modify: `crates/ion-skill/src/search.rs`

**Step 1: Write the test**

Add to the `tests` module:

```rust
    #[test]
    fn github_source_parses_search_response() {
        let json = r#"{
            "total_count": 2,
            "items": [
                {
                    "full_name": "anthropics/skills",
                    "description": "AI agent skills collection",
                    "html_url": "https://github.com/anthropics/skills"
                },
                {
                    "full_name": "acme/brainstorm-skill",
                    "description": "Brainstorm skill",
                    "html_url": "https://github.com/acme/brainstorm-skill"
                }
            ]
        }"#;
        let results = parse_github_response(json, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "anthropics/skills");
        assert_eq!(results[0].source, "anthropics/skills");
        assert_eq!(results[0].registry, "github");
        assert_eq!(results[1].description, "Brainstorm skill");
    }

    #[test]
    fn github_source_handles_empty_items() {
        let json = r#"{"total_count": 0, "items": []}"#;
        let results = parse_github_response(json, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn github_source_null_description() {
        let json = r#"{
            "total_count": 1,
            "items": [{"full_name": "a/b", "description": null, "html_url": "https://github.com/a/b"}]
        }"#;
        let results = parse_github_response(json, 10).unwrap();
        assert_eq!(results[0].description, "");
    }
```

**Step 2: Run tests, verify they fail**

Run: `cargo test -p ion-skill search`
Expected: FAIL — `parse_github_response` not found

**Step 3: Implement GitHubSource**

Add to `search.rs`, above the `tests` module:

```rust
#[derive(Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
}

#[derive(Deserialize)]
struct GitHubRepo {
    full_name: String,
    description: Option<String>,
}

/// Parse a GitHub search API response into SearchResults.
pub fn parse_github_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let resp: GitHubSearchResponse =
        serde_json::from_str(body).map_err(|e| crate::Error::Search(format!("Invalid GitHub response: {e}")))?;
    Ok(resp
        .items
        .into_iter()
        .take(limit)
        .map(|repo| SearchResult {
            name: repo.full_name.clone(),
            description: repo.description.unwrap_or_default(),
            source: repo.full_name,
            registry: "github".to_string(),
        })
        .collect())
}

/// Searches GitHub repositories for skills.
pub struct GitHubSource;

impl SearchSource for GitHubSource {
    fn name(&self) -> &str {
        "github"
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        let url = format!(
            "https://api.github.com/search/repositories?q={query}+topic:ai-skills&per_page={limit}"
        );

        let client = reqwest::blocking::Client::new();
        let mut request = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ion-skill-manager");

        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .map_err(|e| crate::Error::Http(format!("GitHub: {e}")))?;
        let body = response
            .text()
            .map_err(|e| crate::Error::Http(format!("GitHub: {e}")))?;
        parse_github_response(&body, limit)
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill search`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/ion-skill/src/search.rs
git commit -m "feat: implement GitHubSource for searching GitHub repositories"
```

---

### Task 6: Implement AgentSource

**Files:**
- Modify: `crates/ion-skill/src/search.rs`

**Step 1: Write the test**

Add to the `tests` module:

```rust
    #[test]
    fn parse_agent_output_tab_separated() {
        let output = "brainstorming\tCollaborative brainstorming\tanthropics/skills/brainstorming\n\
                       tdd\tTest driven development\tanthropics/skills/tdd\n";
        let results = parse_agent_output(output, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "brainstorming");
        assert_eq!(results[0].description, "Collaborative brainstorming");
        assert_eq!(results[0].source, "anthropics/skills/brainstorming");
        assert_eq!(results[0].registry, "agent");
    }

    #[test]
    fn parse_agent_output_freeform_becomes_single_result() {
        let output = "I found some skills about brainstorming that might help.";
        let results = parse_agent_output(output, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "agent-result");
        assert!(results[0].description.contains("brainstorming"));
    }

    #[test]
    fn parse_agent_output_empty() {
        let results = parse_agent_output("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn parse_agent_output_mixed_lines() {
        let output = "brainstorming\tDesc\towner/repo\nsome freeform text\ntdd\tDesc2\towner2/repo2\n";
        let results = parse_agent_output(output, 10);
        // Tab-separated lines parsed as structured, freeform lines skipped when mixed with structured
        assert_eq!(results.len(), 2);
    }
```

**Step 2: Run tests, verify they fail**

Run: `cargo test -p ion-skill search`
Expected: FAIL — `parse_agent_output` not found

**Step 3: Implement AgentSource**

Add to `search.rs`:

```rust
/// Parse agent CLI output. If lines are tab-separated (name\tdesc\tsource), parse as structured.
/// If no structured lines found, return the whole output as a single freeform result.
pub fn parse_agent_output(output: &str, limit: usize) -> Vec<SearchResult> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    let mut structured: Vec<SearchResult> = Vec::new();
    for line in trimmed.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() == 3 {
            structured.push(SearchResult {
                name: parts[0].trim().to_string(),
                description: parts[1].trim().to_string(),
                source: parts[2].trim().to_string(),
                registry: "agent".to_string(),
            });
        }
    }

    if structured.is_empty() {
        // Freeform output — return as a single result
        vec![SearchResult {
            name: "agent-result".to_string(),
            description: trimmed.to_string(),
            source: String::new(),
            registry: "agent".to_string(),
        }]
    } else {
        structured.into_iter().take(limit).collect()
    }
}

/// Searches by shelling out to a user-configured CLI agent command.
pub struct AgentSource {
    pub command_template: String,
}

impl SearchSource for AgentSource {
    fn name(&self) -> &str {
        "agent"
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        let command = self.command_template.replace("{query}", query);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| crate::Error::Search(format!("Failed to run agent command: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Search(format!("Agent command failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_agent_output(&stdout, limit))
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill search`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/ion-skill/src/search.rs
git commit -m "feat: implement AgentSource for CLI agent search"
```

---

### Task 7: Add registries and search config sections

**Files:**
- Modify: `crates/ion-skill/src/config.rs`

**Step 1: Write the test**

Add to the `tests` module in `config.rs`:

```rust
    #[test]
    fn load_registries_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, r#"
[registries.skills-sh]
url = "https://skills.sh/api"
default = true

[registries.my-company]
url = "https://skills.internal.co/api"

[search]
agent-command = "claude -p 'search: {query}'"
"#).unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(config.registries.len(), 2);
        assert_eq!(config.registries["skills-sh"].url, "https://skills.sh/api");
        assert_eq!(config.registries["skills-sh"].default, Some(true));
        assert_eq!(config.search.agent_command, Some("claude -p 'search: {query}'".to_string()));
    }

    #[test]
    fn load_config_without_registries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\n").unwrap();

        let config = GlobalConfig::load_from(&path).unwrap();
        assert!(config.registries.is_empty());
        assert_eq!(config.search.agent_command, None);
    }

    #[test]
    fn get_value_registries_and_search() {
        let mut config = GlobalConfig::default();
        config.registries.insert("skills-sh".to_string(), RegistryConfig {
            url: "https://skills.sh/api".to_string(),
            default: Some(true),
        });
        config.search.agent_command = Some("claude search {query}".to_string());

        assert_eq!(config.get_value("registries.skills-sh"), Some("https://skills.sh/api".to_string()));
        assert_eq!(config.get_value("search.agent-command"), Some("claude search {query}".to_string()));
    }
```

**Step 2: Run tests, verify they fail**

Run: `cargo test -p ion-skill config`
Expected: FAIL — `registries` field not found on `GlobalConfig`

**Step 3: Implement the new config sections**

Add new structs in `config.rs` (after `UiConfig`):

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RegistryConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SearchConfig {
    pub agent_command: Option<String>,
}
```

Add fields to `GlobalConfig`:

```rust
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryConfig>,
    #[serde(default)]
    pub search: SearchConfig,
```

Update `get_value` to handle the new sections — add these match arms:

```rust
            "registries" => self.registries.get(field).map(|r| r.url.clone()),
            "search" => match field {
                "agent-command" => self.search.agent_command.clone(),
                _ => None,
            },
```

Update `set_value_in_file` — add `"registries" | "search"` to the valid section match:

```rust
            "targets" | "sources" | "cache" | "ui" | "registries" | "search" => {}
```

And add handling for `search.agent-command` in the set match (string type, falls through to default).

For `registries.*`, setting a value sets the URL:

```rust
            ("registries", _) => {
                // Setting a registry URL — create inline table { url = "..." }
                use toml_edit::InlineTable;
                let mut t = InlineTable::new();
                t.insert("url", value.into());
                doc[section][field] = toml_edit::Item::Value(toml_edit::Value::InlineTable(t));
            }
```

Update `list_values` to include new sections:

```rust
        for (k, v) in &self.registries {
            entries.push((format!("registries.{k}"), v.url.clone()));
        }
        if let Some(ref cmd) = self.search.agent_command {
            entries.push(("search.agent-command".to_string(), cmd.clone()));
        }
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill config`
Expected: all tests pass

**Step 5: Verify existing tests still pass**

Run: `cargo test -p ion-skill`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/ion-skill/src/config.rs
git commit -m "feat: add registries and search sections to GlobalConfig"
```

---

### Task 8: Implement search orchestration (cascade + parallel)

**Files:**
- Modify: `crates/ion-skill/src/search.rs`

This adds the `run_search` function that orchestrates cascade (default) and parallel (`--all`) modes.

**Step 1: Write the test**

Add to `tests` module:

```rust
    #[test]
    fn cascade_stops_at_first_source_with_results() {
        let sources: Vec<Box<dyn SearchSource>> = vec![
            Box::new(FakeSource { results: vec![] }),
            Box::new(FakeSource {
                results: vec![SearchResult {
                    name: "found".to_string(),
                    description: "".to_string(),
                    source: "x/y".to_string(),
                    registry: "second".to_string(),
                }],
            }),
            Box::new(FakeSource {
                results: vec![SearchResult {
                    name: "should-not-reach".to_string(),
                    description: "".to_string(),
                    source: "a/b".to_string(),
                    registry: "third".to_string(),
                }],
            }),
        ];
        let results = cascade_search(sources, "q", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "found");
    }

    #[test]
    fn cascade_returns_empty_if_all_sources_empty() {
        let sources: Vec<Box<dyn SearchSource>> = vec![
            Box::new(FakeSource { results: vec![] }),
            Box::new(FakeSource { results: vec![] }),
        ];
        let results = cascade_search(sources, "q", 10);
        assert!(results.is_empty());
    }
```

**Step 2: Run tests, verify they fail**

Run: `cargo test -p ion-skill search`
Expected: FAIL — `cascade_search` not found

**Step 3: Implement cascade_search**

Add to `search.rs`:

```rust
/// Run search sources sequentially. Stop at the first source that returns results.
/// If a source errors, print a warning and continue.
pub fn cascade_search(
    sources: Vec<Box<dyn SearchSource>>,
    query: &str,
    limit: usize,
) -> Vec<SearchResult> {
    for source in &sources {
        match source.search(query, limit) {
            Ok(results) if !results.is_empty() => return results,
            Ok(_) => {} // empty, try next
            Err(e) => {
                eprintln!("Warning: {} search failed: {e}", source.name());
            }
        }
    }
    vec![]
}

/// Run all search sources in parallel using threads. Merge all results.
/// If a source errors, print a warning and skip it.
pub fn parallel_search(
    sources: Vec<Box<dyn SearchSource + Send>>,
    query: &str,
    limit: usize,
) -> Vec<SearchResult> {
    let query = query.to_string();
    let handles: Vec<_> = sources
        .into_iter()
        .map(|source| {
            let q = query.clone();
            std::thread::spawn(move || {
                match source.search(&q, limit) {
                    Ok(results) => results,
                    Err(e) => {
                        eprintln!("Warning: {} search failed: {e}", source.name());
                        vec![]
                    }
                }
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for handle in handles {
        if let Ok(results) = handle.join() {
            all_results.extend(results);
        }
    }
    all_results
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill search`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/ion-skill/src/search.rs
git commit -m "feat: implement cascade and parallel search orchestration"
```

---

### Task 9: Add the search CLI command

**Files:**
- Create: `src/commands/search.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add the command module**

In `src/commands/mod.rs`, add:

```rust
pub mod search;
```

**Step 2: Add the Search variant to Commands enum**

In `src/main.rs`, add to the `Commands` enum:

```rust
    /// Search for skills across registries and GitHub
    Search {
        /// Search query (word or phrase)
        query: String,
        /// Search all sources in parallel instead of cascading
        #[arg(long, short)]
        all: bool,
        /// Include configured CLI agent in search
        #[arg(long)]
        agent: bool,
        /// Pick a result to install interactively
        #[arg(long, short)]
        interactive: bool,
        /// Search only a specific source
        #[arg(long)]
        source: Option<String>,
        /// Max results per source
        #[arg(long, default_value = "10")]
        limit: usize,
    },
```

Add the match arm in `main()`:

```rust
        Commands::Search { query, all, agent, interactive, source, limit } => {
            commands::search::run(&query, all, agent, interactive, source.as_deref(), limit)
        }
```

**Step 3: Implement the search command**

Create `src/commands/search.rs`:

```rust
use ion_skill::config::GlobalConfig;
use ion_skill::search::{
    cascade_search, parallel_search, AgentSource, GitHubSource, RegistrySource, SearchResult,
    SearchSource,
};

pub fn run(
    query: &str,
    all: bool,
    agent: bool,
    interactive: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<()> {
    let config = GlobalConfig::load()?;
    let results = execute_search(&config, query, all, agent, source_filter, limit)?;

    if results.is_empty() {
        println!("No results found for '{query}'.");
        return Ok(());
    }

    print_results(&results);

    if interactive {
        pick_and_install(&results)?;
    }

    Ok(())
}

fn execute_search(
    config: &GlobalConfig,
    query: &str,
    all: bool,
    agent: bool,
    source_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    if let Some(name) = source_filter {
        return search_single_source(config, name, query, agent, limit);
    }

    if all {
        let mut sources = build_sources_send(config);
        if agent {
            if let Some(s) = build_agent_source(config) {
                sources.push(Box::new(s));
            }
        }
        Ok(parallel_search(sources, query, limit))
    } else {
        let mut sources = build_sources(config);
        if agent {
            if let Some(s) = build_agent_source(config) {
                sources.push(Box::new(s));
            }
        }
        Ok(cascade_search(sources, query, limit))
    }
}

fn build_sources(config: &GlobalConfig) -> Vec<Box<dyn SearchSource>> {
    let mut sources: Vec<Box<dyn SearchSource>> = Vec::new();

    // Default registries first, then others
    for (name, reg) in &config.registries {
        if reg.default == Some(true) {
            sources.insert(
                0,
                Box::new(RegistrySource {
                    registry_name: name.clone(),
                    base_url: reg.url.clone(),
                }),
            );
        } else {
            sources.push(Box::new(RegistrySource {
                registry_name: name.clone(),
                base_url: reg.url.clone(),
            }));
        }
    }

    sources.push(Box::new(GitHubSource));
    sources
}

fn build_sources_send(config: &GlobalConfig) -> Vec<Box<dyn SearchSource + Send>> {
    let mut sources: Vec<Box<dyn SearchSource + Send>> = Vec::new();

    for (name, reg) in &config.registries {
        if reg.default == Some(true) {
            sources.insert(
                0,
                Box::new(RegistrySource {
                    registry_name: name.clone(),
                    base_url: reg.url.clone(),
                }),
            );
        } else {
            sources.push(Box::new(RegistrySource {
                registry_name: name.clone(),
                base_url: reg.url.clone(),
            }));
        }
    }

    sources.push(Box::new(GitHubSource));
    sources
}

fn build_agent_source(config: &GlobalConfig) -> Option<AgentSource> {
    config.search.agent_command.as_ref().map(|cmd| AgentSource {
        command_template: cmd.clone(),
    })
}

fn search_single_source(
    config: &GlobalConfig,
    name: &str,
    query: &str,
    agent: bool,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    if name == "github" {
        return Ok(GitHubSource.search(query, limit)?);
    }
    if name == "agent" && agent {
        if let Some(s) = build_agent_source(config) {
            return Ok(s.search(query, limit)?);
        }
        anyhow::bail!("No agent-command configured. Set it with: ion config set search.agent-command '<command>'");
    }
    if let Some(reg) = config.registries.get(name) {
        let source = RegistrySource {
            registry_name: name.to_string(),
            base_url: reg.url.clone(),
        };
        return Ok(source.search(query, limit)?);
    }
    anyhow::bail!("Unknown source '{name}'. Available: {}", available_sources(config));
}

fn available_sources(config: &GlobalConfig) -> String {
    let mut names: Vec<&str> = config.registries.keys().map(|s| s.as_str()).collect();
    names.push("github");
    names.join(", ")
}

fn print_results(results: &[SearchResult]) {
    let mut current_registry = String::new();
    for r in results {
        if r.registry != current_registry {
            if !current_registry.is_empty() {
                println!();
            }
            let count = results.iter().filter(|x| x.registry == r.registry).count();
            println!(
                " {} ({} result{})",
                r.registry,
                count,
                if count == 1 { "" } else { "s" }
            );
            current_registry = r.registry.clone();
        }
        if r.source.is_empty() {
            // Freeform agent output
            println!("  {}", r.description);
        } else {
            println!(
                "  {:<24} {:<44} {}",
                r.name, r.description, r.source
            );
        }
    }
}

fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    let installable: Vec<&SearchResult> = results.iter().filter(|r| !r.source.is_empty()).collect();
    if installable.is_empty() {
        println!("No installable results to select from.");
        return Ok(());
    }

    let items: Vec<String> = installable
        .iter()
        .map(|r| format!("{} — {}", r.name, r.description))
        .collect();

    let selection = dialoguer::Select::new()
        .with_prompt("Select a skill to install")
        .items(&items)
        .default(0)
        .interact_opt()?;

    if let Some(idx) = selection {
        let chosen = installable[idx];
        println!("\nInstalling '{}'...", chosen.name);
        let status = std::process::Command::new("ion")
            .arg("add")
            .arg(&chosen.source)
            .status()?;
        if !status.success() {
            anyhow::bail!("ion add failed");
        }
    }

    Ok(())
}
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles

**Step 5: Commit**

```bash
git add src/commands/search.rs src/commands/mod.rs src/main.rs
git commit -m "feat: add ion search command with cascade, parallel, and interactive modes"
```

---

### Task 10: Add integration tests

**Files:**
- Create: `tests/search_integration.rs`

**Step 1: Write integration tests**

Create `tests/search_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn search_shows_help() {
    let output = ion_cmd()
        .args(["search", "--help"])
        .output()
        .expect("failed to run ion");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Search for skills"));
    assert!(stdout.contains("--all"));
    assert!(stdout.contains("--agent"));
    assert!(stdout.contains("--interactive"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--limit"));
}

#[test]
fn search_no_registries_falls_through_gracefully() {
    // With no config, search should attempt GitHub and either succeed or warn
    // This test verifies the command doesn't panic
    let output = ion_cmd()
        .args(["search", "nonexistent-skill-xyz-12345"])
        .output()
        .expect("failed to run ion");
    // Should not crash (exit 0 or 1 with error message, not panic)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"));
}

#[test]
fn search_unknown_source_errors() {
    let output = ion_cmd()
        .args(["search", "test", "--source", "nonexistent"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown source"));
}
```

**Step 2: Run tests**

Run: `cargo test --test search_integration`
Expected: all tests pass

**Step 3: Commit**

```bash
git add tests/search_integration.rs
git commit -m "test: add integration tests for search command"
```

---

### Task 11: Final verification

**Step 1: Run all tests**

Run: `cargo test`
Expected: all tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

**Step 3: Verify the command works end-to-end**

Run: `cargo run -- search --help`
Expected: shows help text with all flags

Run: `cargo run -- search brainstorming`
Expected: searches (may warn about no registries configured, falls through to GitHub)
