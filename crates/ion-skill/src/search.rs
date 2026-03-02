use serde::Deserialize;

/// Search result from any source.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub source: String,
    pub registry: String,
    pub stars: Option<u64>,
    pub skill_description: Option<String>,
}

impl SearchResult {
    pub fn new(name: impl Into<String>, description: impl Into<String>, source: impl Into<String>, registry: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            source: source.into(),
            registry: registry.into(),
            stars: None,
            skill_description: None,
        }
    }
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
        .map(|e| SearchResult::new(e.name, e.description, e.source, registry_name))
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
        let url = format!("{}/search", self.base_url.trim_end_matches('/'));
        log::debug!("registry '{}': GET {} (q={query}, limit={limit})", self.registry_name, url);
        let response = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?
            .get(&url)
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send()
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?
            .error_for_status()
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?;
        let body = response
            .text()
            .map_err(|e| crate::Error::Http(format!("{}: {e}", self.registry_name)))?;
        log::debug!("registry '{}': received {} bytes", self.registry_name, body.len());
        let results = parse_registry_response(&body, &self.registry_name, limit)?;
        log::debug!("registry '{}': parsed {} results", self.registry_name, results.len());
        Ok(results)
    }
}

// --- GitHub CLI (`gh`) search ---

/// JSON entry from `gh search code --json path,repository`
#[derive(Deserialize)]
struct GhCodeEntry {
    path: String,
    repository: GhCodeRepo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhCodeRepo {
    name_with_owner: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    stargazers_count: Option<u64>,
}

/// Parse `gh search code --json` output into SearchResults.
/// Deduplicates by repository (a repo may have multiple SKILL.md matches).
pub fn parse_gh_code_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<GhCodeEntry> =
        serde_json::from_str(body).map_err(|e| crate::Error::Search(format!("Invalid gh output: {e}")))?;
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for item in entries {
        if seen.insert(item.repository.name_with_owner.clone()) {
            let source = if item.path == "SKILL.md" {
                item.repository.name_with_owner.clone()
            } else {
                // SKILL.md is in a subdirectory — include the path minus the filename
                let skill_dir = item.path.trim_end_matches("/SKILL.md").trim_end_matches("SKILL.md");
                let skill_dir = skill_dir.trim_end_matches('/');
                if skill_dir.is_empty() {
                    item.repository.name_with_owner.clone()
                } else {
                    format!("{}/{}", item.repository.name_with_owner, skill_dir)
                }
            };
            let mut result = SearchResult::new(
                item.repository.name_with_owner.clone(),
                item.repository.description.unwrap_or_default(),
                source,
                "github",
            );
            result.stars = item.repository.stargazers_count;
            results.push(result);
            if results.len() >= limit {
                break;
            }
        }
    }
    Ok(results)
}

/// JSON entry from `gh search repos --json fullName,description`
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRepoEntry {
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    stargazers_count: Option<u64>,
}

/// Parse `gh search repos --json` output into SearchResults.
pub fn parse_gh_repo_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<GhRepoEntry> =
        serde_json::from_str(body).map_err(|e| crate::Error::Search(format!("Invalid gh output: {e}")))?;
    Ok(entries
        .into_iter()
        .take(limit)
        .map(|repo| {
            let mut result = SearchResult::new(
                repo.full_name.clone(),
                repo.description.unwrap_or_default(),
                repo.full_name,
                "github",
            );
            result.stars = repo.stargazers_count;
            result
        })
        .collect())
}

/// Searches GitHub using the `gh` CLI. Uses `gh search code --filename SKILL.md`
/// for precise results, falling back to `gh search repos` if code search returns nothing.
/// If `gh` is not installed, returns an error suggesting installation.
pub struct GitHubSource;

impl GitHubSource {
    fn gh_available() -> bool {
        std::process::Command::new("gh")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn run_gh(args: &[&str]) -> crate::Result<String> {
        log::debug!("github: running gh {}", args.join(" "));
        let output = std::process::Command::new("gh")
            .args(args)
            .output()
            .map_err(|e| crate::Error::Search(format!("Failed to run gh: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Search(format!("gh failed: {stderr}")));
        }
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        log::debug!("github: gh returned {} bytes", stdout.len());
        Ok(stdout)
    }
}

impl SearchSource for GitHubSource {
    fn name(&self) -> &str {
        "github"
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        if !Self::gh_available() {
            return Err(crate::Error::Search(
                "GitHub CLI (gh) not found. Install it from https://cli.github.com and run `gh auth login`".to_string(),
            ));
        }

        let limit_str = limit.to_string();

        // Try code search for SKILL.md files first (most precise)
        log::debug!("github: trying code search for SKILL.md files matching {query:?}");
        match Self::run_gh(&[
            "search", "code", "--filename", "SKILL.md", query,
            "--json", "path,repository", "--limit", &limit_str,
        ]) {
            Ok(body) => {
                let results = parse_gh_code_response(&body, limit)?;
                if !results.is_empty() {
                    log::debug!("github: code search found {} results", results.len());
                    return Ok(results);
                }
                log::debug!("github: code search found 0 results, falling back to repo search");
            }
            Err(e) => {
                log::debug!("github: code search failed ({e}), falling back to repo search");
            }
        }

        // Fall back to repo search
        let repo_query = format!("{query} skill");
        log::debug!("github: repo search for {repo_query:?}");
        let body = Self::run_gh(&[
            "search", "repos", &repo_query,
            "--json", "fullName,description,stargazersCount", "--limit", &limit_str,
        ])?;
        let results = parse_gh_repo_response(&body, limit)?;
        log::debug!("github: repo search found {} results", results.len());
        Ok(results)
    }
}

/// Enrich GitHub search results by fetching SKILL.md descriptions from each repository.
/// Results are enriched in parallel using threads.
pub fn enrich_github_results(results: &mut [SearchResult]) {
    let handles: Vec<_> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.registry == "github" && !r.source.is_empty())
        .map(|(i, r)| {
            let source = r.source.clone();
            std::thread::spawn(move || {
                (i, fetch_skill_description(&source), fetch_stars_if_missing(&source))
            })
        })
        .collect();

    for handle in handles {
        if let Ok((i, skill_desc, stars)) = handle.join() {
            if let Some(desc) = skill_desc {
                results[i].skill_description = Some(desc);
            }
            if let Some(s) = stars {
                if results[i].stars.is_none() {
                    results[i].stars = Some(s);
                }
            }
        }
    }
}

/// Fetch the description from a SKILL.md file in a GitHub repository.
fn fetch_skill_description(source: &str) -> Option<String> {
    // source is like "owner/repo" or "owner/repo/path/to/skill"
    let parts: Vec<&str> = source.splitn(3, '/').collect();
    if parts.len() < 2 {
        return None;
    }
    let (owner_repo, skill_path) = if parts.len() == 3 {
        (format!("{}/{}", parts[0], parts[1]), format!("{}/SKILL.md", parts[2]))
    } else {
        (source.to_string(), "SKILL.md".to_string())
    };

    log::debug!("enrich: fetching SKILL.md from {owner_repo} path={skill_path}");
    let output = std::process::Command::new("gh")
        .args(["api", &format!("repos/{owner_repo}/contents/{skill_path}"), "--jq", ".content"])
        .output()
        .ok()?;

    if !output.status.success() {
        log::debug!("enrich: failed to fetch SKILL.md from {owner_repo}");
        return None;
    }

    let b64 = String::from_utf8_lossy(&output.stdout);
    let b64_clean: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&b64_clean)?;
    parse_skill_description(&decoded)
}

/// Fetch star count for a repo if not already known.
fn fetch_stars_if_missing(source: &str) -> Option<u64> {
    let parts: Vec<&str> = source.splitn(3, '/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner_repo = format!("{}/{}", parts[0], parts[1]);
    let output = std::process::Command::new("gh")
        .args(["api", &format!("repos/{owner_repo}"), "--jq", ".stargazers_count"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Simple base64 decoder (standard alphabet, no padding required).
fn base64_decode(input: &str) -> Option<String> {
    // Simple lookup table approach
    let table: Vec<u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter().copied().collect();
    let mut buf = Vec::new();
    let mut bits: u32 = 0;
    let mut n_bits = 0;
    for &byte in input.as_bytes() {
        if byte == b'=' { break; }
        let val = table.iter().position(|&b| b == byte)? as u32;
        bits = (bits << 6) | val;
        n_bits += 6;
        if n_bits >= 8 {
            n_bits -= 8;
            buf.push((bits >> n_bits) as u8);
            bits &= (1 << n_bits) - 1;
        }
    }
    String::from_utf8(buf).ok()
}

/// Parse YAML frontmatter from SKILL.md content to extract the description.
fn parse_skill_description(content: &str) -> Option<String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("---")?;
    let frontmatter = &rest[..end];
    // Simple line-based YAML parsing for description field
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
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
            structured.push(SearchResult::new(
                parts[0].trim(),
                parts[1].trim(),
                parts[2].trim(),
                "agent",
            ));
        }
    }
    if structured.is_empty() {
        vec![SearchResult::new("agent-result", trimmed, "", "agent")]
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
        let escaped = format!("'{}'", query.replace('\'', "'\\''"));
        let command = self.command_template.replace("{query}", &escaped);
        log::debug!("agent: executing command: {command}");
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| crate::Error::Search(format!("Failed to run agent command: {e}")))?;
        log::debug!("agent: exit status={}", output.status);
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!("agent: stderr={stderr}");
            return Err(crate::Error::Search(format!("Agent command failed: {stderr}")));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::debug!("agent: stdout={} bytes", stdout.len());
        let results = parse_agent_output(&stdout, limit);
        log::debug!("agent: parsed {} results", results.len());
        Ok(results)
    }
}

/// Run search sources sequentially. Stop at the first source that returns results.
/// If a source errors, print a warning and continue.
pub fn cascade_search(
    sources: Vec<Box<dyn SearchSource + Send>>,
    query: &str,
    limit: usize,
) -> Vec<SearchResult> {
    for source in &sources {
        log::debug!("cascade: trying source '{}'", source.name());
        match source.search(query, limit) {
            Ok(results) if !results.is_empty() => {
                log::debug!("cascade: source '{}' returned {} results, stopping", source.name(), results.len());
                return results;
            }
            Ok(_) => {
                log::debug!("cascade: source '{}' returned 0 results, continuing", source.name());
            }
            Err(e) => {
                log::debug!("cascade: source '{}' failed: {e}", source.name());
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
    log::debug!("parallel: spawning {} search threads", sources.len());
    let query = query.to_string();
    let handles: Vec<_> = sources
        .into_iter()
        .map(|source| {
            let q = query.clone();
            std::thread::spawn(move || {
                log::debug!("parallel: thread searching '{}'", source.name());
                match source.search(&q, limit) {
                    Ok(results) => {
                        log::debug!("parallel: '{}' returned {} results", source.name(), results.len());
                        results
                    }
                    Err(e) => {
                        log::debug!("parallel: '{}' failed: {e}", source.name());
                        eprintln!("Warning: {} search failed: {e}", source.name());
                        vec![]
                    }
                }
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(results) => all_results.extend(results),
            Err(_) => eprintln!("Warning: a search thread panicked"),
        }
    }
    log::debug!("parallel: merged {} total results", all_results.len());
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
            results: vec![SearchResult::new("test-skill", "A test", "owner/repo/test-skill", "fake")],
        };
        let results = source.search("test", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }

    #[test]
    fn trait_search_respects_limit() {
        let source = FakeSource {
            results: vec![
                SearchResult::new("a", "", "", "fake"),
                SearchResult::new("b", "", "", "fake"),
                SearchResult::new("c", "", "", "fake"),
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
    fn gh_repo_search_parses_response() {
        let json = r#"[
            {"fullName": "anthropics/skills", "description": "AI agent skills collection"},
            {"fullName": "acme/brainstorm-skill", "description": "Brainstorm skill"}
        ]"#;
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "anthropics/skills");
        assert_eq!(results[0].source, "anthropics/skills");
        assert_eq!(results[0].registry, "github");
        assert_eq!(results[1].description, "Brainstorm skill");
    }

    #[test]
    fn gh_repo_search_handles_empty() {
        let json = "[]";
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn gh_repo_search_null_description() {
        let json = r#"[{"fullName": "a/b", "description": null}]"#;
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert_eq!(results[0].description, "");
    }

    #[test]
    fn gh_code_search_parses_response() {
        let json = r#"[
            {"path": "SKILL.md", "repository": {"nameWithOwner": "org/skill-a", "description": "Skill A"}},
            {"path": "skills/brainstorming/SKILL.md", "repository": {"nameWithOwner": "org/monorepo", "description": "Multi-skill repo"}}
        ]"#;
        let results = parse_gh_code_response(json, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "org/skill-a");
        assert_eq!(results[0].source, "org/skill-a");
        assert_eq!(results[1].name, "org/monorepo");
        assert_eq!(results[1].source, "org/monorepo/skills/brainstorming");
    }

    #[test]
    fn gh_code_search_deduplicates_repos() {
        let json = r#"[
            {"path": "skills/a/SKILL.md", "repository": {"nameWithOwner": "org/repo", "description": "Repo"}},
            {"path": "skills/b/SKILL.md", "repository": {"nameWithOwner": "org/repo", "description": "Repo"}},
            {"path": "SKILL.md", "repository": {"nameWithOwner": "org/other", "description": "Other"}}
        ]"#;
        let results = parse_gh_code_response(json, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source, "org/repo/skills/a");
        assert_eq!(results[1].source, "org/other");
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
        let sources: Vec<Box<dyn SearchSource + Send>> = vec![
            Box::new(FakeSource { results: vec![] }),
            Box::new(FakeSource {
                results: vec![SearchResult::new("found", "", "x/y", "second")],
            }),
            Box::new(FakeSource {
                results: vec![SearchResult::new("should-not-reach", "", "a/b", "third")],
            }),
        ];
        let results = cascade_search(sources, "q", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "found");
    }

    #[test]
    fn cascade_returns_empty_if_all_sources_empty() {
        let sources: Vec<Box<dyn SearchSource + Send>> = vec![
            Box::new(FakeSource { results: vec![] }),
            Box::new(FakeSource { results: vec![] }),
        ];
        let results = cascade_search(sources, "q", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn parse_skill_description_from_frontmatter() {
        let content = "---\nname: brainstorming\ndescription: Collaborative brainstorming skill\n---\n# Brainstorming\nContent here.";
        assert_eq!(parse_skill_description(content), Some("Collaborative brainstorming skill".to_string()));
    }

    #[test]
    fn parse_skill_description_quoted() {
        let content = "---\nname: test\ndescription: \"A quoted description\"\n---\n";
        assert_eq!(parse_skill_description(content), Some("A quoted description".to_string()));
    }

    #[test]
    fn parse_skill_description_missing() {
        let content = "---\nname: test\n---\n# No description";
        assert_eq!(parse_skill_description(content), None);
    }

    #[test]
    fn parse_skill_description_no_frontmatter() {
        let content = "# Just a markdown file\nNo frontmatter here.";
        assert_eq!(parse_skill_description(content), None);
    }

    #[test]
    fn base64_decode_works() {
        // "Hello, World!" in base64
        assert_eq!(base64_decode("SGVsbG8sIFdvcmxkIQ=="), Some("Hello, World!".to_string()));
    }

    #[test]
    fn base64_decode_no_padding() {
        // "Hi" in base64 (no padding needed)
        assert_eq!(base64_decode("SGk"), Some("Hi".to_string()));
    }

    #[test]
    fn gh_repo_search_includes_stars() {
        let json = r#"[{"fullName": "org/repo", "description": "A repo", "stargazersCount": 42}]"#;
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert_eq!(results[0].stars, Some(42));
    }

    #[test]
    fn gh_repo_search_missing_stars() {
        let json = r#"[{"fullName": "org/repo", "description": "A repo"}]"#;
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert_eq!(results[0].stars, None);
    }
}
