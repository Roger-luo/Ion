use serde::Deserialize;

/// Extract "owner/repo" from a source string.
/// `"obra/superpowers/skills/brainstorming"` → `"obra/superpowers"`.
/// Returns the full string if it has fewer than two `/`-separated segments.
pub fn owner_repo_of(source: &str) -> &str {
    let mut slashes = source.match_indices('/');
    if let Some((_, _)) = slashes.next() {
        if let Some((second, _)) = slashes.next() {
            return &source[..second];
        }
        // Exactly one slash: "owner/repo" — return as-is
        return source;
    }
    source
}

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

// --- skills.sh website scraping ---

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

    log::debug!("skills.sh: parsed {} total skills, filtering by {query:?}", entries.len());

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
            // Build source: if skillId differs from the repo name, it's a monorepo skill
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
        let response = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| crate::Error::Http(format!("skills.sh: {e}")))?
            .get("https://skills.sh/")
            .send()
            .map_err(|e| crate::Error::Http(format!("skills.sh: {e}")))?
            .error_for_status()
            .map_err(|e| crate::Error::Http(format!("skills.sh: {e}")))?;
        let body = response
            .text()
            .map_err(|e| crate::Error::Http(format!("skills.sh: {e}")))?;
        log::debug!("skills.sh: received {} bytes", body.len());
        let results = parse_skills_sh_page(&body, query, limit);
        log::debug!("skills.sh: {} results for {query:?}", results.len());
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
/// Deduplicates by install source (repo + skill path), so monorepos with
/// multiple skills each get their own result.
/// Filters out entries that aren't actual SKILL.md files (GitHub's --filename
/// does substring matching, so "browser-agent-skill.md" would slip through).
pub fn parse_gh_code_response(body: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
    let entries: Vec<GhCodeEntry> =
        serde_json::from_str(body).map_err(|e| crate::Error::Search(format!("Invalid gh output: {e}")))?;
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for item in entries {
        // Only accept files literally named SKILL.md (not "my-skill.md", "SOFT_SKILL.md", etc.)
        let filename = item.path.rsplit('/').next().unwrap_or(&item.path);
        if !filename.eq_ignore_ascii_case("skill.md") {
            continue;
        }

        let source = if item.path == "SKILL.md" || item.path == "skill.md" {
            item.repository.name_with_owner.clone()
        } else {
            // SKILL.md is in a subdirectory — include the path minus the filename
            let skill_dir = item.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
            if skill_dir.is_empty() {
                item.repository.name_with_owner.clone()
            } else {
                format!("{}/{}", item.repository.name_with_owner, skill_dir)
            }
        };
        if seen.insert(source.clone()) {
            // Use the skill directory name as the display name for monorepo skills
            let name = if source.contains('/') && source != item.repository.name_with_owner {
                // Monorepo skill: show "skill-dir (owner/repo)" for clarity
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

        // Collect results from all strategies with generous internal limits,
        // then deduplicate, sort by stars, and truncate to the requested limit.
        // This ensures high-star repos (like monorepos with many skills) surface
        // first regardless of which strategy found them.
        let fetch_limit = (limit * 3).max(30).to_string();
        let mut results = Vec::new();
        let mut seen_sources = std::collections::HashSet::new();

        // 1. Code search (content): find SKILL.md files whose content matches the query
        log::debug!("github: code search (content) for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search", "code", "--filename", "SKILL.md", query,
            "--json", "path,repository", "--limit", &fetch_limit,
        ]) {
            for r in parse_gh_code_response(&body, limit * 3)? {
                if seen_sources.insert(r.source.clone()) {
                    results.push(r);
                }
            }
            log::debug!("github: content search found {} results", results.len());
        }

        // 2. Code search (path): find SKILL.md files in directories matching the query
        //    e.g., searching "brainstorming" finds skills/brainstorming/SKILL.md
        log::debug!("github: code search (path) for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search", "code", "--filename", "SKILL.md", "--match", "path", query,
            "--json", "path,repository", "--limit", &fetch_limit,
        ]) {
            let before = results.len();
            for r in parse_gh_code_response(&body, limit * 3)? {
                if seen_sources.insert(r.source.clone()) {
                    results.push(r);
                }
            }
            log::debug!("github: path search added {} results", results.len() - before);
        }

        // 3. Repo search: find repos whose name/description matches the query.
        //    For skill-related repos, enumerate ALL their individual skills.
        log::debug!("github: repo search for {query:?}");
        if let Ok(body) = Self::run_gh(&[
            "search", "repos", query,
            "--json", "fullName,description,stargazersCount", "--limit", "10",
        ]) {
            let repo_results = parse_gh_repo_response(&body, 10)?;
            log::debug!("github: repo search found {} repos", repo_results.len());
            for repo in &repo_results {
                // Skip repos already fully represented in code search results
                if seen_sources.contains(&repo.source) {
                    continue;
                }
                // Enumerate individual skills within skill-related repos
                if looks_skill_related(repo) {
                    let mut skills = enumerate_repo_skills(&repo.source, limit * 3, &seen_sources);
                    if !skills.is_empty() {
                        log::debug!("github: enumerated {} skills in {}", skills.len(), repo.source);
                        // Propagate the repo's star count to enumerated skills
                        // (gh search code doesn't include stargazersCount)
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
                        log::debug!("github: {} looks skill-related but has no SKILL.md, skipping", repo.source);
                    }
                    continue;
                }
                // Fall back to showing repo as a single result only if it has a SKILL.md
                if seen_sources.insert(repo.source.clone()) && repo_has_skill_md(&repo.source) {
                    results.push(repo.clone());
                }
            }
        }

        // Sort by stars (descending), then select with repo diversity
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
        results = select_with_diversity(results, limit);

        log::debug!("github: returning {} results", results.len());
        Ok(results)
    }
}

/// Check if a search result looks skill-related (has "skill" or "agent" in name/description).
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
/// Returns results for each skill found, excluding already-seen sources.
fn enumerate_repo_skills(
    repo: &str,
    limit: usize,
    seen: &std::collections::HashSet<String>,
) -> Vec<SearchResult> {
    let limit_str = limit.to_string();
    log::debug!("github: enumerating skills in {repo}");
    let body = match GitHubSource::run_gh(&[
        "search", "code", "--filename", "SKILL.md", "--repo", repo,
        "--json", "path,repository", "--limit", &limit_str,
    ]) {
        Ok(b) => b,
        Err(e) => {
            log::debug!("github: failed to enumerate {repo}: {e}");
            return vec![];
        }
    };
    match parse_gh_code_response(&body, limit) {
        Ok(results) => results.into_iter().filter(|r| !seen.contains(&r.source)).collect(),
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
        "search", "code", "--filename", "SKILL.md", "--repo", repo,
        "--json", "path,repository", "--limit", "1",
    ]) else {
        return false;
    };
    parse_gh_code_response(&body, 1).is_ok_and(|r| !r.is_empty())
}

/// Select up to `limit` results while ensuring repo diversity.
///
/// Guarantees that the final set contains results from at least `MIN_REPOS`
/// different repos (or all available repos if fewer exist). When a single
/// high-star repo would otherwise dominate, lower-priority skills from that
/// repo are evicted to make room for at least one result from other repos.
///
/// Expects `results` to be pre-sorted by stars descending.
fn select_with_diversity(mut results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
    const MIN_REPOS: usize = 5;

    if results.len() <= limit {
        return results;
    }

    // Split into selected (top `limit`) and overflow
    let overflow = results.split_off(limit);

    // Count unique repos already in selected set
    let selected_repos: std::collections::HashSet<&str> =
        results.iter().map(|r| owner_repo_of(&r.source)).collect();

    if selected_repos.len() >= MIN_REPOS {
        return results;
    }

    // Collect one representative from each new repo in the overflow
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

    // Evict from over-represented repos to make room
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
        // Find repo with the most results
        let max_repo = repo_counts
            .iter()
            .max_by_key(|&(_, c)| *c)
            .map(|(k, _)| k.clone());
        if let Some(repo) = max_repo {
            if repo_counts[&repo] <= 1 {
                break; // Can't remove without losing a repo entirely
            }
            // Remove the last (lowest-star) result from this repo
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
    results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
    results
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
            if let Some(s) = stars
                && results[i].stars.is_none()
            {
                results[i].stars = Some(s);
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
        .to_vec();
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

    #[test]
    fn owner_repo_of_full_path() {
        assert_eq!(owner_repo_of("obra/superpowers/skills/brainstorming"), "obra/superpowers");
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
        // 8 skills from monorepo (high stars) + 3 from other repos (low stars)
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

        // Pre-sort by stars (as GitHubSource does)
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));

        let selected = select_with_diversity(results, 10);
        assert_eq!(selected.len(), 10);

        // Should have results from at least 4 repos (mono + other/a + other/b + other/c)
        let repos: std::collections::HashSet<&str> =
            selected.iter().map(|r| owner_repo_of(&r.source)).collect();
        assert!(repos.len() >= 4, "expected >=4 repos, got {}: {:?}", repos.len(), repos);
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
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));

        let selected = select_with_diversity(results, 10);
        // Should still be sorted by stars descending
        for w in selected.windows(2) {
            assert!(w[0].stars.unwrap_or(0) >= w[1].stars.unwrap_or(0));
        }
    }

    fn make_result(name: &str, source: &str, stars: u64) -> SearchResult {
        let mut r = SearchResult::new(name, "", source, "github");
        r.stars = Some(stars);
        r
    }

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
        // "obra" matches both superpowers skills by source
        let results = parse_skills_sh_page(body, "obra", 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn skills_sh_parse_escaped_quotes() {
        // Simulates RSC payload with escaped quotes
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
        // When skillId matches the repo name, source should just be owner/repo
        let body = r#"x"initialSkills":[{"source":"acme/tdd","skillId":"tdd","name":"tdd","installs":100}]y"#;
        let results = parse_skills_sh_page(body, "tdd", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "acme/tdd");
    }
}
