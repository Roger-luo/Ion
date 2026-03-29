---
title: "ionem::shell::gh"
description: "GitHub CLI wrappers."
order: 999
---

GitHub CLI wrappers.

Use [`require()`] to verify `gh` is installed, then call methods on the
returned [`Gh`] handle:

```ignore
let gh = gh::require()?;

gh.search_code("brainstorm")
    .filename("SKILL.md")
    .json(&["path", "repository"])
    .limit(30)
    .run()?;

gh.api("repos/owner/repo/contents/SKILL.md")
    .jq(".content")
    .run()?;
```

## Gh

A validated handle proving the `gh` CLI is available.

Obtained via [`require()`]. All builder entry points live here.

### Methods

#### `run`

```rust
pub fn run(&self, args: &[&str]) -> Result<String>
```

Run an arbitrary `gh` command with the given args. Returns stdout.

#### `star_repo`

```rust
pub fn star_repo(&self, repo: &str) -> Result<()>
```

Star a GitHub repository.

#### `search_code`

```rust
pub fn search_code(&self, query: &str) -> SearchCode
```

Start building a `gh search code` command.

#### `search_repos`

```rust
pub fn search_repos(&self, query: &str) -> SearchRepos
```

Start building a `gh search repos` command.

#### `api`

```rust
pub fn api(&self, endpoint: impl Into<String>) -> Api
```

Start building a `gh api` command.

---

## SearchCode

Builder for `gh search code`.

*…and private fields*

### Methods

#### `filename`

```rust
pub fn filename(self, name: impl Into<String>) -> Self
```

Filter by filename (e.g. `"SKILL.md"`).

#### `match_on`

```rust
pub fn match_on(self, field: impl Into<String>) -> Self
```

Restrict which field to match the query against (`"path"` or `"file"`).

#### `repo`

```rust
pub fn repo(self, repo: impl Into<String>) -> Self
```

Restrict search to a specific repository.

#### `json`

```rust
pub fn json(self, fields: &[&str]) -> Self
```

Select JSON output fields (comma-joined for `--json`).

#### `limit`

```rust
pub fn limit(self, n: usize) -> Self
```

Maximum number of results.

#### `run`

```rust
pub fn run(self) -> Result<String>
```

Execute the command and return stdout.

---

## SearchRepos

Builder for `gh search repos`.

*…and private fields*

### Methods

#### `json`

```rust
pub fn json(self, fields: &[&str]) -> Self
```

Select JSON output fields (comma-joined for `--json`).

#### `limit`

```rust
pub fn limit(self, n: usize) -> Self
```

Maximum number of results.

#### `run`

```rust
pub fn run(self) -> Result<String>
```

Execute the command and return stdout.

---

## Api

Builder for `gh api`.

*…and private fields*

### Methods

#### `jq`

```rust
pub fn jq(self, expr: impl Into<String>) -> Self
```

Apply a jq expression to the response.

#### `run`

```rust
pub fn run(self) -> Result<String>
```

Execute the command and return stdout.

---

## require

```rust
pub fn require() -> Result<Gh>
```

Verify `gh` is installed and return a handle to run commands.

---

## available

```rust
pub fn available() -> bool
```

Check if `gh` CLI is installed.

---

## run

```rust
pub fn run(args: &[&str]) -> Result<String>
```

Run an arbitrary `gh` command with the given args. Returns stdout.

---

## star_repo

```rust
pub fn star_repo(repo: &str) -> Result<()>
```

Star a GitHub repository.

---

## search_code

```rust
pub fn search_code(query: &str) -> SearchCode
```

Start building a `gh search code` command.

---

## search_repos

```rust
pub fn search_repos(query: &str) -> SearchRepos
```

Start building a `gh search repos` command.

---

## api

```rust
pub fn api(endpoint: impl Into<String>) -> Api
```

Start building a `gh api` command.

