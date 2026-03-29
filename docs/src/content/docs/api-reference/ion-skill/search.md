---
title: "ion-skill::search"
description: ""
order: 999
---

## SearchResult

Search result from any source.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `description` | `String` |  |
| `source` | `String` |  |
| `registry` | `String` |  |
| `stars` | `Option<u64>` |  |
| `skill_description` | `Option<String>` |  |

### Methods

#### `new`

```rust
pub fn new(name: impl Into<String>, description: impl Into<String>, source: impl Into<String>, registry: impl Into<String>) -> Self
```

#### `sort_by_stars`

```rust
pub fn sort_by_stars(results: &mut [Self])
```

Sort results by stars descending (missing stars treated as 0).

#### `sort_by_relevance`

```rust
pub fn sort_by_relevance(results: &mut [Self], query: &str)
```

Sort results by relevance to the query, combining text match quality
with popularity (stars). Exact name/source matches rank highest,
then prefix matches, then substring matches, with stars as tiebreaker.

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`
- `Deserialize<'de>`

---

## owner_repo_of

```rust
pub fn owner_repo_of(source: &str) -> &str
```

Extract "owner/repo" from a source string.
`"obra/superpowers/skills/brainstorming"` → `"obra/superpowers"`.
Returns the full string if it has fewer than two `/`-separated segments.

---

## skill_dir_name

```rust
pub fn skill_dir_name(source: &str) -> &str
```

Extract the leaf skill directory name from a source path.
`"obra/superpowers/skills/brainstorming"` → `"brainstorming"`.
Returns the full source if it has no path beyond `owner/repo`.

---

## group_by_owner_repo

```rust
pub fn group_by_owner_repo(results: &[SearchResult]) -> Vec<(String, Vec<usize>)>
```

Group results by `owner_repo_of`, preserving first-occurrence order.
Returns `(owner_repo, indices_into_results)` pairs.

---

## parallel_search

```rust
pub fn parallel_search(sources: Vec<Box<dyn >>, query: &str, limit: usize, cache: Option<&SearchCache>, max_age_secs: u64) -> Vec<SearchResult>
```

Run all search sources in parallel using threads. Merge all results.
If a source errors, print a warning and skip it.

When `cache` is provided, each source checks the cache before making a
network call and writes results back on a miss. The "agent" source is
never cached because its output is dynamic.

---

## cascade_search

```rust
pub fn cascade_search(sources: Vec<Box<dyn >>, query: &str, limit: usize) -> Vec<SearchResult>
```

Run search sources sequentially. Stop at the first source that returns results.
If a source errors, print a warning and continue.

