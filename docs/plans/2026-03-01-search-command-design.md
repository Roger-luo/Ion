# Search Command Design

## Overview

Add `ion search <query>` to discover skills across multiple sources: the skills.sh registry, custom registries, GitHub, and optionally an external CLI agent.

## CLI Interface

```
ion search <query> [flags]
```

**Arguments:**
- `<query>` — search term (word or quoted string)

**Flags:**
- `--all` / `-a` — search all sources in parallel instead of cascading
- `--agent` — include configured CLI agent in the search
- `-i` / `--interactive` — pick from results to install via `ion add`
- `--source <name>` — search only a specific source
- `--limit <n>` — max results per source (default: 10)

**Output:**
```
 skills.sh (3 results)
  brainstorming          Collaborative design brainstorming skill    anthropics/skills/brainstorming
  brainstorm-lite        Lightweight brainstorming                   acme/brainstorm-lite

 my-registry (1 result)
  team-brainstorm        Internal team brainstorming skill           internal/team-brainstorm
```

Results grouped by source, showing name, description, and installable source string.

## Search Cascade

**Default (sequential):**
1. skills.sh registry (HTTP API)
2. Custom registries (same API contract)
3. GitHub search API
4. Agent CLI (only with `--agent`)

Stop early: if a source returns results, skip remaining sources.

**With `--all`:** Sources 1-3 run in parallel, results merged. Agent still opt-in via `--agent`.

## Source Architecture

```rust
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub source: String,        // installable string for `ion add`
    pub registry: String,      // which source it came from
}

pub trait SearchSource {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
}
```

Four implementations: `SkillsShSource`, `CustomRegistrySource`, `GitHubSource`, `AgentSource`.

## Config Additions

```toml
# ~/.config/ion/config.toml

[registries]
skills-sh = { url = "https://skills.sh/api", default = true }
my-company = { url = "https://skills.internal.co/api" }

[search]
agent-command = "claude -p 'search for AI skills about: {query}'"
```

## Dependencies

- `reqwest` (with `rustls-tls`) — HTTP client for registry and GitHub APIs
- `tokio` — async runtime for parallel search with `--all`
- `serde_json` — parsing API responses
- `dialoguer` — interactive selection for `-i` mode

## GitHub Search

Unauthenticated GitHub search API: `GET /search/repositories?q={query}+topic:ai-skills`. Supports `GITHUB_TOKEN` env var for higher rate limits.

## Agent Source

Shell out via `std::process::Command`. Replace `{query}` in the configured command template. Expected output: newline-delimited, tab-separated (`name\tdescription\tsource`). Free-form text displayed as-is under an "Agent results" header.

## Interactive Mode

With `-i`, display results using `dialoguer` select list. On selection, run `ion add <source>` for the chosen skill.

## Error Handling

If any source fails (network, timeout, parse error), print a warning and continue to next source. One source failing never blocks the entire search.

## File Changes

- `crates/ion-skill/src/search.rs` — `SearchSource` trait and implementations
- `src/commands/search.rs` — command handler
- `src/main.rs` — add `Search` to `Commands` enum
- `crates/ion-skill/src/config.rs` — extend for `[registries]` and `[search]` sections
