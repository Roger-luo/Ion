use serde::Deserialize;

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
            Ok(_) => {}
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
            results: vec![SearchResult {
                name: "test-skill".to_string(),
                description: "A test".to_string(),
                source: "owner/repo/test-skill".to_string(),
                registry: "fake".to_string(),
            }],
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

    #[test]
    fn registry_source_parses_json_response() {
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

    #[test]
    fn github_source_parses_search_response() {
        let json = r#"{
            "total_count": 2,
            "items": [
                {"full_name": "anthropics/skills", "description": "AI agent skills collection", "html_url": "https://github.com/anthropics/skills"},
                {"full_name": "acme/brainstorm-skill", "description": "Brainstorm skill", "html_url": "https://github.com/acme/brainstorm-skill"}
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
        let json = r#"{"total_count": 1, "items": [{"full_name": "a/b", "description": null, "html_url": "https://github.com/a/b"}]}"#;
        let results = parse_github_response(json, 10).unwrap();
        assert_eq!(results[0].description, "");
    }

    #[test]
    fn parse_agent_output_tab_separated() {
        let output = "brainstorming\tCollaborative brainstorming\tanthropics/skills/brainstorming\ntdd\tTest driven development\tanthropics/skills/tdd\n";
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
        assert_eq!(results.len(), 2);
    }

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
}
