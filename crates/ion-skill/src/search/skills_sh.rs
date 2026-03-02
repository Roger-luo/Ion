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
            let source = if e.source.contains('/') {
                let repo_name = e.source.rsplit('/').next().unwrap_or("");
                if e.skill_id != repo_name && !e.skill_id.is_empty() {
                    format!("{}/{}", e.source, e.skill_id)
                } else {
                    e.source.clone()
                }
            } else {
                e.source.clone()
            };

            let mut result = SearchResult::new(e.name, "", source, "skills.sh");
            result.stars = Some(e.installs);
            result
        })
        .collect()
}

/// Searches skills.sh by fetching the website and filtering the embedded skill catalog.
pub struct SkillsShSource;

impl SearchSource for SkillsShSource {
    fn name(&self) -> &str {
        "skills.sh"
    }

    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        log::debug!("skills.sh: fetching website");
        let body = super::http_get("https://skills.sh/", 15, "skills.sh")?;
        log::debug!("skills.sh: received {} bytes", body.len());
        let results = parse_skills_sh_page(&body, query, limit);
        log::debug!("skills.sh: {} results for {query:?}", results.len());
        Ok(results)
    }
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
        assert_eq!(results[0].stars, Some(5000));
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
}
