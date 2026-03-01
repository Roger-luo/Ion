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
}
