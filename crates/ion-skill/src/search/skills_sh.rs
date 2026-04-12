use serde::Deserialize;

use super::{SearchResult, SearchSource};

/// A skill entry parsed from the skills.sh RSC payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsShEntry {
    source: String,
    #[serde(default)]
    skill_id: String,
    name: String,
    #[serde(default)]
    installs: u64,
}

/// Parse the skills.sh HTML page to extract skill entries from the RSC payload.
/// The page embeds an `initialSkills` JSON array inside a `self.__next_f.push(...)` call.
pub fn parse_skills_sh_page(body: &str, query: &str, limit: usize) -> Vec<SearchResult> {
    // The RSC payload may use escaped quotes — unescape before parsing.
    let unescaped;
    let text = if body.contains("\\\"initialSkills\\\"") {
        unescaped = body.replace("\\\"", "\"").replace("\\\\", "\\");
        unescaped.as_str()
    } else {
        body
    };

    let marker = "\"initialSkills\":";
    let Some(start) = text.find(marker) else {
        log::debug!("skills.sh: initialSkills marker not found");
        return vec![];
    };
    let json_start = start + marker.len();

    // Find the matching closing bracket for the array.
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut end = json_start;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &b) in bytes[json_start..].iter().enumerate() {
        if in_string {
            if b == b'"' && !escaped {
                in_string = false;
            }
            escaped = b == b'\\' && !escaped;
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    end = json_start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        log::debug!("skills.sh: failed to find end of initialSkills array");
        return vec![];
    }

    let json_str = &text[json_start..end];

    let entries: Vec<SkillsShEntry> = match serde_json::from_str(json_str) {
        Ok(e) => e,
        Err(e) => {
            log::debug!("skills.sh: failed to parse initialSkills: {e}");
            return vec![];
        }
    };

    log::debug!(
        "skills.sh: parsed {} total skills, filtering by {query:?}",
        entries.len()
    );

    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    entries
        .into_iter()
        .filter(|e| {
            let name_lower = e.name.to_lowercase();
            let source_lower = e.source.to_lowercase();
            let skill_id_lower = e.skill_id.to_lowercase();
            query_words.iter().all(|w| {
                name_lower.contains(w) || source_lower.contains(w) || skill_id_lower.contains(w)
            })
        })
        .take(limit)
        .map(|e| {
            let src = e.source.trim();
            let sid = e.skill_id.trim();
            let source = if src.contains('/') {
                let repo_name = src.rsplit('/').next().unwrap_or("");
                if sid != repo_name && !sid.is_empty() {
                    format!("{src}/{sid}")
                } else {
                    src.to_string()
                }
            } else {
                src.to_string()
            };

            let mut result = SearchResult::new(e.name.trim(), "", source, "skills.sh");
            result.weekly_installs = Some(e.installs);
            result
        })
        .collect()
}

/// JSON response from the skills.sh search API.
#[derive(Deserialize)]
struct SkillsShApiResponse {
    #[serde(default)]
    skills: Vec<SkillsShEntry>,
}

/// Parse the skills.sh search API JSON response.
pub fn parse_skills_sh_api_response(body: &str, limit: usize) -> Vec<SearchResult> {
    let resp: SkillsShApiResponse = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("skills.sh: failed to parse API response: {e}");
            return vec![];
        }
    };

    log::debug!("skills.sh: API returned {} skills", resp.skills.len());

    resp.skills
        .into_iter()
        .take(limit)
        .map(|e| {
            let src = e.source.trim();
            let sid = e.skill_id.trim();
            let source = if src.contains('/') {
                let repo_name = src.rsplit('/').next().unwrap_or("");
                if sid != repo_name && !sid.is_empty() {
                    format!("{src}/{sid}")
                } else {
                    src.to_string()
                }
            } else {
                src.to_string()
            };

            let mut result = SearchResult::new(e.name.trim(), "", source, "skills.sh");
            result.weekly_installs = Some(e.installs);
            result
        })
        .collect()
}

/// Searches skills.sh using its search API, falling back to homepage scraping.
pub struct SkillsShSource;

impl SearchSource for SkillsShSource {
    fn name(&self) -> &str {
        "skills.sh"
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        // Try the search API first
        log::debug!("skills.sh: querying search API for {query:?}");
        match super::http_get_with_query(
            "https://skills.sh/api/search",
            &[("q", query)],
            15,
            "skills.sh",
        ) {
            Ok(body) => {
                let results = parse_skills_sh_api_response(&body, limit);
                if !results.is_empty() {
                    log::debug!("skills.sh: API returned {} results", results.len());
                    return Ok(results);
                }
                log::debug!("skills.sh: API returned 0 results, falling back to page scrape");
            }
            Err(e) => {
                log::debug!("skills.sh: API failed ({e}), falling back to page scrape");
            }
        }

        // Fallback: scrape the homepage
        log::debug!("skills.sh: fetching website");
        let body = super::http_get("https://skills.sh/", 15, "skills.sh")?;
        log::debug!("skills.sh: received {} bytes", body.len());
        let results = parse_skills_sh_page(&body, query, limit);
        log::debug!("skills.sh: {} results for {query:?}", results.len());
        Ok(results)
    }
}

/// Fetch the skill description from the skills.sh detail page.
/// Parses the rendered summary section from the HTML.
pub(super) fn fetch_skills_sh_description(source: &str) -> Option<String> {
    let url = format!("https://skills.sh/{source}");
    let body = super::http_get(&url, 10, "skills.sh-detail").ok()?;
    parse_skills_sh_summary(&body)
}

/// Extract the summary text from a skills.sh skill detail page.
/// The summary is rendered inside a `<div>` after the "SUMMARY" heading,
/// within a prose block whose first `<p><strong>...</strong></p>` holds
/// the one-line description.
fn parse_skills_sh_summary(body: &str) -> Option<String> {
    // The summary text appears as the first <p><strong>TEXT</strong></p> inside
    // the "Summary" section. We look for the pattern in the HTML.
    let marker = "Summary</div>";
    let idx = body.find(marker)?;
    let after = &body[idx + marker.len()..];

    // Find the first <p> content — it may be wrapped in <strong>.
    let p_start = after.find("<p>")?;
    let p_content = &after[p_start + 3..];
    let p_end = p_content.find("</p>")?;
    let inner = &p_content[..p_end];

    // Strip HTML tags to get plain text.
    let text = strip_html_tags(inner);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Simple HTML tag stripper — removes `<...>` sequences.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skills_sh_parse_basic() {
        let body = r#"stuff before"initialSkills":[{"source":"obra/superpowers","skillId":"brainstorming","name":"brainstorming","installs":5000},{"source":"obra/superpowers","skillId":"writing-plans","name":"writing-plans","installs":3000},{"source":"acme/tdd","skillId":"tdd","name":"tdd","installs":1000}]more stuff"#;
        let results = parse_skills_sh_page(body, "brainstorming", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "brainstorming");
        assert_eq!(results[0].source, "obra/superpowers/brainstorming");
        assert_eq!(results[0].registry, "skills.sh");
        assert_eq!(results[0].weekly_installs, Some(5000));
    }

    #[test]
    fn skills_sh_parse_broad_query() {
        let body = r#"x"initialSkills":[{"source":"obra/superpowers","skillId":"brainstorming","name":"brainstorming","installs":5000},{"source":"obra/superpowers","skillId":"writing","name":"writing","installs":3000},{"source":"acme/skill","skillId":"skill","name":"skill","installs":100}]y"#;
        let results = parse_skills_sh_page(body, "obra", 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn skills_sh_parse_escaped_quotes() {
        let body = r#"x\"initialSkills\":[{\"source\":\"owner/repo\",\"skillId\":\"my-skill\",\"name\":\"my-skill\",\"installs\":42}]y"#;
        let results = parse_skills_sh_page(body, "my-skill", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my-skill");
    }

    #[test]
    fn skills_sh_parse_no_marker() {
        let body = "no skills data here";
        let results = parse_skills_sh_page(body, "test", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn skills_sh_same_skill_id_as_repo() {
        let body = r#"x"initialSkills":[{"source":"acme/tdd","skillId":"tdd","name":"tdd","installs":100}]y"#;
        let results = parse_skills_sh_page(body, "tdd", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "acme/tdd");
    }

    #[test]
    fn skills_sh_api_response_parses() {
        let body = r#"{"query":"rust","searchType":"fuzzy","skills":[{"source":"apollographql/skills","skillId":"rust-best-practices","name":"rust-best-practices","installs":6219},{"source":"wshobson/agents","skillId":"rust-async-patterns","name":"rust-async-patterns","installs":7967}],"count":2,"duration_ms":10}"#;
        let results = parse_skills_sh_api_response(body, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "rust-best-practices");
        assert_eq!(
            results[0].source,
            "apollographql/skills/rust-best-practices"
        );
        assert_eq!(results[0].weekly_installs, Some(6219));
        assert_eq!(results[0].registry, "skills.sh");
        assert_eq!(results[1].source, "wshobson/agents/rust-async-patterns");
    }

    #[test]
    fn skills_sh_api_response_respects_limit() {
        let body = r#"{"skills":[{"source":"a/b","skillId":"s1","name":"s1","installs":100},{"source":"a/b","skillId":"s2","name":"s2","installs":50}]}"#;
        let results = parse_skills_sh_api_response(body, 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn skills_sh_api_trims_whitespace() {
        let body = r#"{"skills":[{"source":" wshobson/agents ","skillId":" rust-async-patterns ","name":" rust-async-patterns ","installs":100}]}"#;
        let results = parse_skills_sh_api_response(body, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-async-patterns");
        assert_eq!(results[0].source, "wshobson/agents/rust-async-patterns");
    }

    #[test]
    fn skills_sh_page_trims_whitespace() {
        let body = r#"x"initialSkills":[{"source":" owner/repo ","skillId":" my-skill ","name":" my-skill ","installs":42}]y"#;
        let results = parse_skills_sh_page(body, "my-skill", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my-skill");
        assert_eq!(results[0].source, "owner/repo/my-skill");
    }

    #[test]
    fn parse_summary_basic() {
        let html = r#"<div>Summary</div><div><div><p><strong>Refactor code while preserving behavior.</strong></p></div></div>"#;
        assert_eq!(
            parse_skills_sh_summary(html),
            Some("Refactor code while preserving behavior.".to_string())
        );
    }

    #[test]
    fn parse_summary_no_strong() {
        let html = r#"<div>Summary</div><div><p>A plain description.</p></div>"#;
        assert_eq!(
            parse_skills_sh_summary(html),
            Some("A plain description.".to_string())
        );
    }

    #[test]
    fn parse_summary_missing() {
        let html = "<html><body>No summary section here</body></html>";
        assert_eq!(parse_skills_sh_summary(html), None);
    }

    #[test]
    fn parse_summary_realistic_html() {
        // Matches the actual skills.sh page structure
        let html = r#"<div class="flex items-center gap-2 text-sm font-mono text-white mb-4 pb-4 border-b border-border uppercase">Summary</div><div class="mb-8 rounded-lg border"><div class="prose"><p><strong>Refactor code while preserving behavior, improving clarity, and reducing complexity.</strong></p>
<ul><li>Covers five core refactoring patterns</li></ul></div></div>"#;
        assert_eq!(
            parse_skills_sh_summary(html),
            Some(
                "Refactor code while preserving behavior, improving clarity, and reducing complexity."
                    .to_string()
            )
        );
    }

    #[test]
    fn strip_tags() {
        assert_eq!(strip_html_tags("<strong>bold</strong> text"), "bold text");
        assert_eq!(strip_html_tags("no tags"), "no tags");
        assert_eq!(strip_html_tags("<a href=\"x\">link</a>"), "link");
    }
}
