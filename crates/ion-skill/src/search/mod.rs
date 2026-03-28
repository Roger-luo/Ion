mod agent;
mod cache;
mod github;
mod registry;
mod skills_sh;

pub use agent::{AgentSource, parse_agent_output};
pub use cache::SearchCache;
pub use github::{
    GitHubSource, enrich_github_results, parse_gh_code_response, parse_gh_repo_response,
};
pub use registry::{RegistrySource, parse_registry_response};
pub use skills_sh::{SkillsShSource, parse_skills_sh_page};

/// Extract "owner/repo" from a source string.
/// `"obra/superpowers/skills/brainstorming"` → `"obra/superpowers"`.
/// Returns the full string if it has fewer than two `/`-separated segments.
pub fn owner_repo_of(source: &str) -> &str {
    let mut slashes = source.match_indices('/');
    if let Some((_, _)) = slashes.next() {
        if let Some((second, _)) = slashes.next() {
            return &source[..second];
        }
        return source;
    }
    source
}

/// Extract the leaf skill directory name from a source path.
/// `"obra/superpowers/skills/brainstorming"` → `"brainstorming"`.
/// Returns the full source if it has no path beyond `owner/repo`.
pub fn skill_dir_name(source: &str) -> &str {
    let owner_repo = owner_repo_of(source);
    source
        .strip_prefix(owner_repo)
        .and_then(|s| s.strip_prefix('/'))
        .map(|s| s.rsplit('/').next().unwrap_or(s))
        .unwrap_or(source)
}

/// Search result from any source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub source: String,
    pub registry: String,
    pub stars: Option<u64>,
    pub skill_description: Option<String>,
}

impl SearchResult {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        registry: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            source: source.into(),
            registry: registry.into(),
            stars: None,
            skill_description: None,
        }
    }

    /// Sort results by stars descending (missing stars treated as 0).
    pub fn sort_by_stars(results: &mut [Self]) {
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
    }

    /// Sort results by relevance to the query, combining text match quality
    /// with popularity (stars). Exact name/source matches rank highest,
    /// then prefix matches, then substring matches, with stars as tiebreaker.
    pub fn sort_by_relevance(results: &mut [Self], query: &str) {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        results.sort_by(|a, b| {
            let score_a = relevance_score(a, &query_lower, &query_words);
            let score_b = relevance_score(b, &query_lower, &query_words);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

/// Compute a relevance score for a search result against a query.
/// Higher score = more relevant. Combines text match quality with popularity.
///
/// Scoring:
/// - Exact match on name/source segment:    1000
/// - Query is a prefix of name/source:       500
/// - All query words found in name/source:   200
/// - Substring match in name/source:         100
/// - Match in description:                    50
/// - Popularity bonus: log2(stars + 1)     (0–20ish)
fn relevance_score(result: &SearchResult, query: &str, query_words: &[&str]) -> f64 {
    let name_lower = result.name.to_lowercase();
    let source_lower = result.source.to_lowercase();
    let desc_lower = result.description.to_lowercase();

    // Check each segment of the source path (e.g., "mintlify" in "mintlify/docs")
    let source_segments: Vec<&str> = source_lower.split('/').collect();
    // The leaf name (last segment of source, or skill dir name)
    let leaf = source_segments.last().copied().unwrap_or("");

    let mut text_score: f64 = 0.0;

    // Exact match on leaf name or any source segment
    if leaf == query || source_lower == query || source_segments.contains(&query) {
        text_score = 1000.0;
    }
    // Leaf or segment starts with query
    else if leaf.starts_with(query) || source_segments.iter().any(|s| s.starts_with(query)) {
        text_score = 500.0;
    }
    // All query words appear in name or source
    else if query_words.len() > 1
        && query_words
            .iter()
            .all(|w| name_lower.contains(w) || source_lower.contains(w))
    {
        text_score = 200.0;
    }
    // Substring match in name or source
    else if name_lower.contains(query) || source_lower.contains(query) {
        text_score = 100.0;
    }
    // Match in description only
    else if desc_lower.contains(query) || query_words.iter().all(|w| desc_lower.contains(w)) {
        text_score = 50.0;
    }

    // Popularity bonus: logarithmic so stars don't dominate relevance
    let star_bonus = (result.stars.unwrap_or(0) as f64 + 1.0).log2();

    text_score + star_bonus
}

/// A searchable source of skills.
pub trait SearchSource {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>>;
}

/// Group results by `owner_repo_of`, preserving first-occurrence order.
/// Returns `(owner_repo, indices_into_results)` pairs.
pub fn group_by_owner_repo(results: &[SearchResult]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut key_to_idx: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for (i, r) in results.iter().enumerate() {
        let key = owner_repo_of(&r.source);
        if let Some(&g) = key_to_idx.get(key) {
            groups[g].1.push(i);
        } else {
            key_to_idx.insert(key, groups.len());
            groups.push((key.to_string(), vec![i]));
        }
    }
    groups
}

/// Perform an HTTP GET and return the response body as a string.
fn http_get(url: &str, timeout_secs: u64, label: &str) -> crate::Result<String> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?
        .get(url)
        .send()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?
        .error_for_status()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?;
    response
        .text()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))
}

/// Perform an HTTP GET with query parameters and return the response body.
fn http_get_with_query(
    url: &str,
    query: &[(&str, &str)],
    timeout_secs: u64,
    label: &str,
) -> crate::Result<String> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?
        .get(url)
        .query(query)
        .send()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?
        .error_for_status()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))?;
    response
        .text()
        .map_err(|e| crate::Error::Http(format!("{label}: {e}")))
}

/// Run all search sources in parallel using threads. Merge all results.
/// If a source errors, print a warning and skip it.
///
/// When `cache` is provided, each source checks the cache before making a
/// network call and writes results back on a miss. The "agent" source is
/// never cached because its output is dynamic.
pub fn parallel_search(
    sources: Vec<Box<dyn SearchSource + Send>>,
    query: &str,
    limit: usize,
    cache: Option<&SearchCache>,
    max_age_secs: u64,
) -> Vec<SearchResult> {
    log::debug!("parallel: spawning {} search threads", sources.len());
    let query = query.to_string();

    // Pre-resolve cache hits on the main thread (cache is not Send).
    let source_cache: Vec<_> = sources
        .iter()
        .map(|source| {
            let name = source.name();
            if name == "agent" {
                return None;
            }
            cache.and_then(|c| c.get(name, &query, max_age_secs))
        })
        .collect();

    let handles: Vec<_> = sources
        .into_iter()
        .zip(source_cache)
        .map(|(source, cached)| {
            let q = query.clone();
            std::thread::spawn(move || {
                // Return cached results if available.
                if let Some(results) = cached {
                    log::debug!(
                        "parallel: '{}' using {} cached results",
                        source.name(),
                        results.len()
                    );
                    return (source.name().to_string(), results, false);
                }

                log::debug!("parallel: thread searching '{}'", source.name());
                match source.search(&q, limit) {
                    Ok(results) => {
                        log::debug!(
                            "parallel: '{}' returned {} results",
                            source.name(),
                            results.len()
                        );
                        (source.name().to_string(), results, true)
                    }
                    Err(e) => {
                        log::debug!("parallel: '{}' failed: {e}", source.name());
                        eprintln!("warning: {} search failed: {e}", source.name());
                        (source.name().to_string(), vec![], false)
                    }
                }
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok((source_name, results, from_network)) => {
                // Write fresh network results to cache.
                if from_network
                    && source_name != "agent"
                    && let Some(c) = cache
                {
                    c.put(&source_name, &query, &results);
                }
                all_results.extend(results);
            }
            Err(_) => log::warn!("A search thread panicked"),
        }
    }
    log::debug!("parallel: merged {} total results", all_results.len());
    SearchResult::sort_by_relevance(&mut all_results, &query);

    // Deduplicate across sources: if the same skill source appears from
    // multiple registries (e.g., skills.sh and github), keep the first
    // (highest-relevance) occurrence.
    let mut seen_sources = std::collections::HashSet::new();
    all_results.retain(|r| {
        if r.source.is_empty() {
            return true;
        }
        // Normalize: "obra/superpowers/skills/brainstorming" and
        // "obra/superpowers/brainstorming" should deduplicate.
        // Use owner_repo + leaf skill name as the dedup key.
        let repo = owner_repo_of(&r.source);
        let leaf = skill_dir_name(&r.source);
        let key = if leaf == r.source {
            r.source.clone()
        } else {
            format!("{repo}/{leaf}")
        };
        seen_sources.insert(key)
    });

    log::debug!("parallel: {} results after dedup", all_results.len());
    all_results
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
                log::debug!(
                    "cascade: source '{}' returned {} results, stopping",
                    source.name(),
                    results.len()
                );
                return results;
            }
            Ok(_) => {
                log::debug!(
                    "cascade: source '{}' returned 0 results, continuing",
                    source.name()
                );
            }
            Err(e) => {
                log::debug!("cascade: source '{}' failed: {e}", source.name());
                eprintln!("warning: {} search failed: {e}", source.name());
            }
        }
    }
    vec![]
}

/// Parse YAML frontmatter from SKILL.md content to extract the description.
pub(crate) fn parse_skill_description(content: &str) -> Option<String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("---")?;
    let frontmatter = &rest[..end];
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

/// Simple base64 decoder (standard alphabet, no padding required).
pub(crate) fn base64_decode(input: &str) -> Option<String> {
    const DECODE_TABLE: [u8; 128] = {
        let mut table = [255u8; 128];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[alphabet[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let mut buf = Vec::new();
    let mut bits: u32 = 0;
    let mut n_bits = 0;
    for &byte in input.as_bytes() {
        if byte == b'=' {
            break;
        }
        if byte >= 128 {
            return None;
        }
        let val = DECODE_TABLE[byte as usize];
        if val == 255 {
            return None;
        }
        bits = (bits << 6) | val as u32;
        n_bits += 6;
        if n_bits >= 8 {
            n_bits -= 8;
            buf.push((bits >> n_bits) as u8);
            bits &= (1 << n_bits) - 1;
        }
    }
    String::from_utf8(buf).ok()
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
            results: vec![SearchResult::new(
                "test-skill",
                "A test",
                "owner/repo/test-skill",
                "fake",
            )],
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
    fn owner_repo_of_full_path() {
        assert_eq!(
            owner_repo_of("obra/superpowers/skills/brainstorming"),
            "obra/superpowers"
        );
    }

    #[test]
    fn owner_repo_of_just_owner_repo() {
        assert_eq!(owner_repo_of("obra/superpowers"), "obra/superpowers");
    }

    #[test]
    fn owner_repo_of_no_slash() {
        assert_eq!(owner_repo_of("superpowers"), "superpowers");
    }

    #[test]
    fn owner_repo_of_empty() {
        assert_eq!(owner_repo_of(""), "");
    }

    #[test]
    fn skill_dir_name_full_path() {
        assert_eq!(
            skill_dir_name("obra/superpowers/skills/brainstorming"),
            "brainstorming"
        );
    }

    #[test]
    fn skill_dir_name_just_repo() {
        assert_eq!(skill_dir_name("obra/superpowers"), "obra/superpowers");
    }

    #[test]
    fn parse_skill_description_from_frontmatter() {
        let content = "---\nname: brainstorming\ndescription: Collaborative brainstorming skill\n---\n# Brainstorming\nContent here.";
        assert_eq!(
            parse_skill_description(content),
            Some("Collaborative brainstorming skill".to_string())
        );
    }

    #[test]
    fn parse_skill_description_quoted() {
        let content = "---\nname: test\ndescription: \"A quoted description\"\n---\n";
        assert_eq!(
            parse_skill_description(content),
            Some("A quoted description".to_string())
        );
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
        assert_eq!(
            base64_decode("SGVsbG8sIFdvcmxkIQ=="),
            Some("Hello, World!".to_string())
        );
    }

    #[test]
    fn base64_decode_no_padding() {
        assert_eq!(base64_decode("SGk"), Some("Hi".to_string()));
    }

    #[test]
    fn group_by_owner_repo_groups_correctly() {
        let results = vec![
            SearchResult::new("a", "", "org/repo/a", "test"),
            SearchResult::new("b", "", "org/repo/b", "test"),
            SearchResult::new("c", "", "other/repo", "test"),
        ];
        let groups = group_by_owner_repo(&results);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "org/repo");
        assert_eq!(groups[0].1, vec![0, 1]);
        assert_eq!(groups[1].0, "other/repo");
        assert_eq!(groups[1].1, vec![2]);
    }

    #[test]
    fn relevance_exact_match_beats_high_stars() {
        let mut results = vec![
            {
                let mut r = SearchResult::new(
                    "write-concept",
                    "Writing tool",
                    "someone/write-concept",
                    "github",
                );
                r.stars = Some(5000);
                r
            },
            {
                let mut r = SearchResult::new(
                    "mintlify/docs",
                    "Mintlify docs skill",
                    "mintlify/docs",
                    "github",
                );
                r.stars = Some(10);
                r
            },
        ];
        SearchResult::sort_by_relevance(&mut results, "mintlify");
        assert_eq!(
            results[0].source, "mintlify/docs",
            "exact segment match should rank first"
        );
    }

    #[test]
    fn relevance_prefix_match_beats_description_match() {
        let mut results = vec![
            {
                let mut r = SearchResult::new(
                    "other-tool",
                    "A mintlify integration",
                    "someone/other-tool",
                    "github",
                );
                r.stars = Some(100);
                r
            },
            {
                let mut r = SearchResult::new(
                    "mint-skills",
                    "Skills collection",
                    "user/mint-skills",
                    "github",
                );
                r.stars = Some(5);
                r
            },
        ];
        SearchResult::sort_by_relevance(&mut results, "mint");
        assert_eq!(
            results[0].source, "user/mint-skills",
            "prefix match should rank higher than description match"
        );
    }

    #[test]
    fn relevance_stars_break_ties() {
        let mut results = vec![
            {
                let mut r = SearchResult::new("a", "", "org/a", "github");
                r.stars = Some(10);
                r
            },
            {
                let mut r = SearchResult::new("b", "", "org/b", "github");
                r.stars = Some(1000);
                r
            },
        ];
        // Neither matches "xyz" well, so stars should decide
        SearchResult::sort_by_relevance(&mut results, "xyz");
        assert_eq!(
            results[0].source, "org/b",
            "higher stars should win when text relevance is equal"
        );
    }

    #[test]
    fn sort_by_stars_descending() {
        let mut results = vec![
            {
                let mut r = SearchResult::new("a", "", "", "test");
                r.stars = Some(10);
                r
            },
            {
                let mut r = SearchResult::new("b", "", "", "test");
                r.stars = Some(100);
                r
            },
            SearchResult::new("c", "", "", "test"),
        ];
        SearchResult::sort_by_stars(&mut results);
        assert_eq!(results[0].name, "b");
        assert_eq!(results[1].name, "a");
        assert_eq!(results[2].name, "c");
    }
}
