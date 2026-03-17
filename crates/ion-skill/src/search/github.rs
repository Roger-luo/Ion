use serde::Deserialize;

use super::{SearchResult, SearchSource, base64_decode, owner_repo_of, parse_skill_description};

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

/// Parse `gh search code --json` output into SearchResults.
/// Deduplicates by install source (repo + skill path), so monorepos with
/// multiple skills each get their own result.
/// Filters out entries that aren't actual SKILL.md files.
pub fn parse_gh_code_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<GhCodeEntry> = serde_json::from_str(body)
        .map_err(|e| crate::Error::Search(format!("Invalid gh output: {e}")))?;
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for item in entries {
        let filename = item.path.rsplit('/').next().unwrap_or(&item.path);
        if !filename.eq_ignore_ascii_case("skill.md") {
            continue;
        }

        let source = if item.path == "SKILL.md" || item.path == "skill.md" {
            item.repository.name_with_owner.clone()
        } else {
            let skill_dir = item.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
            if skill_dir.is_empty() {
                item.repository.name_with_owner.clone()
            } else {
                format!("{}/{}", item.repository.name_with_owner, skill_dir)
            }
        };
        if seen.insert(source.clone()) {
            let name = if source.contains('/') && source != item.repository.name_with_owner {
                let skill_dir = source.rsplit('/').next().unwrap_or(&source);
                format!("{} ({})", skill_dir, item.repository.name_with_owner)
            } else {
                item.repository.name_with_owner.clone()
            };
            let mut result = SearchResult::new(
                name,
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

/// Parse `gh search repos --json` output into SearchResults.
pub fn parse_gh_repo_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<GhRepoEntry> = serde_json::from_str(body)
        .map_err(|e| crate::Error::Search(format!("Invalid gh output: {e}")))?;
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

/// Searches GitHub using the `gh` CLI.
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

        let fetch_limit = (limit * 3).max(30).to_string();
        let mut results = Vec::new();
        let mut seen_sources = std::collections::HashSet::new();

        // 1. Code search (content): find SKILL.md files whose content matches the query
        log::debug!("github: code search (content) for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search",
            "code",
            "--filename",
            "SKILL.md",
            query,
            "--json",
            "path,repository",
            "--limit",
            &fetch_limit,
        ]) {
            for r in parse_gh_code_response(&body, limit * 3)? {
                if seen_sources.insert(r.source.clone()) {
                    results.push(r);
                }
            }
            log::debug!("github: content search found {} results", results.len());
        }

        // 2. Code search (path): find SKILL.md files in directories matching the query
        log::debug!("github: code search (path) for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search",
            "code",
            "--filename",
            "SKILL.md",
            "--match",
            "path",
            query,
            "--json",
            "path,repository",
            "--limit",
            &fetch_limit,
        ]) {
            let before = results.len();
            for r in parse_gh_code_response(&body, limit * 3)? {
                if seen_sources.insert(r.source.clone()) {
                    results.push(r);
                }
            }
            log::debug!(
                "github: path search added {} results",
                results.len() - before
            );
        }

        // 3. Repo search: find repos whose name/description matches the query
        log::debug!("github: repo search for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search",
            "repos",
            query,
            "--json",
            "fullName,description,stargazersCount",
            "--limit",
            "10",
        ]) {
            let repo_results = parse_gh_repo_response(&body, 10)?;
            log::debug!("github: repo search found {} repos", repo_results.len());
            for repo in &repo_results {
                if seen_sources.contains(&repo.source) {
                    continue;
                }
                if looks_skill_related(repo) {
                    let mut skills = enumerate_repo_skills(&repo.source, limit * 3, &seen_sources);
                    if !skills.is_empty() {
                        log::debug!(
                            "github: enumerated {} skills in {}",
                            skills.len(),
                            repo.source
                        );
                        for r in &mut skills {
                            if r.stars.is_none() {
                                r.stars = repo.stars;
                            }
                        }
                        for r in skills {
                            seen_sources.insert(r.source.clone());
                            results.push(r);
                        }
                    } else {
                        log::debug!(
                            "github: {} looks skill-related but has no SKILL.md, skipping",
                            repo.source
                        );
                    }
                    continue;
                }
                if seen_sources.insert(repo.source.clone()) && repo_has_skill_md(&repo.source) {
                    results.push(repo.clone());
                }
            }
        }

        // Sort by stars (descending), then select with repo diversity
        SearchResult::sort_by_stars(&mut results);
        results = select_with_diversity(results, limit);

        log::debug!("github: returning {} results", results.len());
        Ok(results)
    }
}

/// Check if a search result looks skill-related.
fn looks_skill_related(result: &SearchResult) -> bool {
    let name_lower = result.name.to_lowercase();
    let desc_lower = result.description.to_lowercase();
    name_lower.contains("skill")
        || name_lower.contains("agent")
        || name_lower.contains("superpower")
        || desc_lower.contains("skill")
        || desc_lower.contains("agent")
}

/// Enumerate individual skills in a GitHub repo by searching for SKILL.md files.
fn enumerate_repo_skills(
    repo: &str,
    limit: usize,
    seen: &std::collections::HashSet<String>,
) -> Vec<SearchResult> {
    let limit_str = limit.to_string();
    log::debug!("github: enumerating skills in {repo}");
    let body = match GitHubSource::run_gh(&[
        "search",
        "code",
        "--filename",
        "SKILL.md",
        "--repo",
        repo,
        "--json",
        "path,repository",
        "--limit",
        &limit_str,
    ]) {
        Ok(b) => b,
        Err(e) => {
            log::debug!("github: failed to enumerate {repo}: {e}");
            return vec![];
        }
    };
    match parse_gh_code_response(&body, limit) {
        Ok(results) => results
            .into_iter()
            .filter(|r| !seen.contains(&r.source))
            .collect(),
        Err(e) => {
            log::debug!("github: failed to parse enumeration for {repo}: {e}");
            vec![]
        }
    }
}

/// Check whether a repo has a SKILL.md at its root.
fn repo_has_skill_md(repo: &str) -> bool {
    log::debug!("github: checking if {repo} has SKILL.md");
    let Ok(body) = GitHubSource::run_gh(&[
        "search",
        "code",
        "--filename",
        "SKILL.md",
        "--repo",
        repo,
        "--json",
        "path,repository",
        "--limit",
        "1",
    ]) else {
        return false;
    };
    parse_gh_code_response(&body, 1).is_ok_and(|r| !r.is_empty())
}

/// Select up to `limit` results while ensuring repo diversity.
fn select_with_diversity(mut results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
    const MIN_REPOS: usize = 5;

    if results.len() <= limit {
        return results;
    }

    let overflow = results.split_off(limit);

    let selected_repos: std::collections::HashSet<&str> =
        results.iter().map(|r| owner_repo_of(&r.source)).collect();

    if selected_repos.len() >= MIN_REPOS {
        return results;
    }

    let mut new_repo_results: Vec<SearchResult> = Vec::new();
    let mut seen_new: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in overflow {
        let repo = owner_repo_of(&r.source).to_string();
        if !selected_repos.contains(repo.as_str()) && seen_new.insert(repo) {
            new_repo_results.push(r);
            if selected_repos.len() + new_repo_results.len() >= MIN_REPOS {
                break;
            }
        }
    }

    if new_repo_results.is_empty() {
        return results;
    }

    let to_add = new_repo_results.len();
    let mut repo_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for r in &results {
        *repo_counts
            .entry(owner_repo_of(&r.source).to_string())
            .or_default() += 1;
    }

    let mut removed = 0;
    while removed < to_add {
        let max_repo = repo_counts
            .iter()
            .max_by_key(|&(_, c)| *c)
            .map(|(k, _)| k.clone());
        if let Some(repo) = max_repo {
            if repo_counts[&repo] <= 1 {
                break;
            }
            if let Some(pos) = results
                .iter()
                .rposition(|r| owner_repo_of(&r.source) == repo)
            {
                results.remove(pos);
                *repo_counts.get_mut(&repo).unwrap() -= 1;
                removed += 1;
            }
        } else {
            break;
        }
    }

    results.extend(new_repo_results.into_iter().take(removed));
    SearchResult::sort_by_stars(&mut results);
    results
}

/// Enrich GitHub search results by fetching SKILL.md descriptions from each repository.
pub fn enrich_github_results(results: &mut [SearchResult]) {
    let handles: Vec<_> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.registry == "github" && !r.source.is_empty())
        .map(|(i, r)| {
            let source = r.source.clone();
            let has_stars = r.stars.is_some();
            std::thread::spawn(move || {
                let desc = fetch_skill_description(&source);
                let stars = if has_stars {
                    None
                } else {
                    fetch_stars(&source)
                };
                (i, desc, stars)
            })
        })
        .collect();

    for handle in handles {
        if let Ok((i, skill_desc, stars)) = handle.join() {
            if let Some(desc) = skill_desc {
                results[i].skill_description = Some(desc);
            }
            if let Some(s) = stars {
                results[i].stars = Some(s);
            }
        }
    }
}

/// Fetch the description from a SKILL.md file in a GitHub repository.
fn fetch_skill_description(source: &str) -> Option<String> {
    let repo = owner_repo_of(source);
    if repo.is_empty() || !repo.contains('/') {
        return None;
    }

    let skill_path = source
        .strip_prefix(repo)
        .and_then(|s| s.strip_prefix('/'))
        .map(|s| format!("{s}/SKILL.md"))
        .unwrap_or_else(|| "SKILL.md".to_string());

    log::debug!("enrich: fetching SKILL.md from {repo} path={skill_path}");
    let output = std::process::Command::new("gh")
        .args([
            "api",
            &format!("repos/{repo}/contents/{skill_path}"),
            "--jq",
            ".content",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        log::debug!("enrich: failed to fetch SKILL.md from {repo}");
        return None;
    }

    let b64 = String::from_utf8_lossy(&output.stdout);
    let b64_clean: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&b64_clean)?;
    parse_skill_description(&decoded)
}

/// Fetch star count for a repo.
fn fetch_stars(source: &str) -> Option<u64> {
    let repo = owner_repo_of(source);
    if repo.is_empty() || !repo.contains('/') {
        return None;
    }

    let output = std::process::Command::new("gh")
        .args(["api", &format!("repos/{repo}"), "--jq", ".stargazers_count"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(name: &str, source: &str, stars: u64) -> SearchResult {
        let mut r = SearchResult::new(name, "", source, "github");
        r.stars = Some(stars);
        r
    }

    #[test]
    fn gh_repo_search_parses_response() {
        let json = r#"[
            {"fullName": "obra/superpowers", "description": "AI agent skills collection"},
            {"fullName": "acme/brainstorm-skill", "description": "Brainstorm skill"}
        ]"#;
        let results = parse_gh_repo_response(json, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "obra/superpowers");
        assert_eq!(results[0].source, "obra/superpowers");
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
        assert_eq!(results[1].name, "brainstorming (org/monorepo)");
        assert_eq!(results[1].source, "org/monorepo/skills/brainstorming");
    }

    #[test]
    fn gh_code_search_shows_all_monorepo_skills() {
        let json = r#"[
            {"path": "skills/a/SKILL.md", "repository": {"nameWithOwner": "org/repo", "description": "Repo"}},
            {"path": "skills/b/SKILL.md", "repository": {"nameWithOwner": "org/repo", "description": "Repo"}},
            {"path": "SKILL.md", "repository": {"nameWithOwner": "org/other", "description": "Other"}}
        ]"#;
        let results = parse_gh_code_response(json, 10).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].source, "org/repo/skills/a");
        assert_eq!(results[0].name, "a (org/repo)");
        assert_eq!(results[1].source, "org/repo/skills/b");
        assert_eq!(results[1].name, "b (org/repo)");
        assert_eq!(results[2].source, "org/other");
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

    #[test]
    fn diversity_noop_when_under_limit() {
        let results = vec![
            make_result("a/one", "a/one", 100),
            make_result("b/two", "b/two", 50),
        ];
        let selected = select_with_diversity(results.clone(), 10);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn diversity_evicts_from_dominant_repo() {
        let mut results = Vec::new();
        for i in 0..8 {
            results.push(make_result(
                &format!("skill-{i} (mono/repo)"),
                &format!("mono/repo/skills/skill-{i}"),
                1000,
            ));
        }
        results.push(make_result("other/a", "other/a", 10));
        results.push(make_result("other/b", "other/b", 5));
        results.push(make_result("other/c", "other/c", 1));

        SearchResult::sort_by_stars(&mut results);

        let selected = select_with_diversity(results, 10);
        assert_eq!(selected.len(), 10);

        let repos: std::collections::HashSet<&str> =
            selected.iter().map(|r| owner_repo_of(&r.source)).collect();
        assert!(
            repos.len() >= 4,
            "expected >=4 repos, got {}: {:?}",
            repos.len(),
            repos
        );
        assert!(repos.contains("other/a"));
        assert!(repos.contains("other/b"));
        assert!(repos.contains("other/c"));
    }

    #[test]
    fn diversity_preserves_order() {
        let mut results = Vec::new();
        for i in 0..12 {
            results.push(make_result(
                &format!("skill-{i}"),
                &format!("big/repo/skills/s{i}"),
                1000,
            ));
        }
        results.push(make_result("small/a", "small/a", 500));
        results.push(make_result("small/b", "small/b", 200));
        SearchResult::sort_by_stars(&mut results);

        let selected = select_with_diversity(results, 10);
        for w in selected.windows(2) {
            assert!(w[0].stars.unwrap_or(0) >= w[1].stars.unwrap_or(0));
        }
    }
}
