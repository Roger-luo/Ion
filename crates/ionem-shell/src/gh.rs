//! GitHub CLI wrappers.
//!
//! Use [`require()`] to verify `gh` is installed, then call methods on the
//! returned [`Gh`] handle:
//!
//! ```ignore
//! let gh = gh::require()?;
//!
//! gh.search_code("brainstorm")
//!     .filename("SKILL.md")
//!     .json(&["path", "repository"])
//!     .limit(30)
//!     .run()?;
//!
//! gh.api("repos/owner/repo/contents/SKILL.md")
//!     .jq(".content")
//!     .run()?;
//! ```

use crate::{Cli, Result};

/// The `gh` CLI descriptor.
pub const CLI: Cli = Cli {
    name: "gh",
    hint: "Install from https://cli.github.com and run `gh auth login`",
};

/// Verify `gh` is installed and return a handle to run commands.
pub fn require() -> Result<Gh> {
    CLI.require()?;
    Ok(Gh)
}

/// A validated handle proving the `gh` CLI is available.
///
/// Obtained via [`require()`]. All builder entry points live here.
pub struct Gh;

impl Gh {
    /// Run an arbitrary `gh` command with the given args. Returns stdout.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run(args)
    }

    /// Star a GitHub repository.
    pub fn star_repo(&self, repo: &str) -> Result<()> {
        star_repo(repo)
    }

    /// Start building a `gh search code` command.
    pub fn search_code(&self, query: &str) -> SearchCode {
        search_code(query)
    }

    /// Start building a `gh search repos` command.
    pub fn search_repos(&self, query: &str) -> SearchRepos {
        search_repos(query)
    }

    /// Start building a `gh api` command.
    pub fn api(&self, endpoint: impl Into<String>) -> Api {
        api(endpoint)
    }
}

// ---------------------------------------------------------------------------
// Free functions (internal / backward-compat — check lazily on spawn)
// ---------------------------------------------------------------------------

/// Check if `gh` CLI is installed.
pub fn available() -> bool {
    CLI.available()
}

/// Run an arbitrary `gh` command with the given args. Returns stdout.
pub fn run(args: &[&str]) -> Result<String> {
    log::debug!("gh: running gh {}", args.join(" "));
    let output = CLI.run_command(CLI.command().args(args))?;
    log::debug!("gh: returned {} bytes", output.len());
    Ok(output)
}

/// Star a GitHub repository.
pub fn star_repo(repo: &str) -> Result<()> {
    CLI.run_status(
        CLI.command()
            .args(["repo", "star", repo])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )
}

// ---------------------------------------------------------------------------
// Builder: gh search code
// ---------------------------------------------------------------------------

/// Start building a `gh search code` command.
pub fn search_code(query: &str) -> SearchCode {
    SearchCode {
        query: query.to_string(),
        filename: None,
        match_on: None,
        repo: None,
        json_fields: None,
        limit: None,
    }
}

/// Builder for `gh search code`.
pub struct SearchCode {
    query: String,
    filename: Option<String>,
    match_on: Option<String>,
    repo: Option<String>,
    json_fields: Option<String>,
    limit: Option<usize>,
}

impl SearchCode {
    /// Filter by filename (e.g. `"SKILL.md"`).
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Restrict which field to match the query against (`"path"` or `"file"`).
    pub fn match_on(mut self, field: impl Into<String>) -> Self {
        self.match_on = Some(field.into());
        self
    }

    /// Restrict search to a specific repository.
    pub fn repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }

    /// Select JSON output fields (comma-joined for `--json`).
    pub fn json(mut self, fields: &[&str]) -> Self {
        self.json_fields = Some(fields.join(","));
        self
    }

    /// Maximum number of results.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the command and return stdout.
    pub fn run(self) -> Result<String> {
        let mut args = vec!["search".to_string(), "code".to_string()];
        if let Some(f) = &self.filename {
            args.push("--filename".to_string());
            args.push(f.clone());
        }
        if let Some(m) = &self.match_on {
            args.push("--match".to_string());
            args.push(m.clone());
        }
        if let Some(r) = &self.repo {
            args.push("--repo".to_string());
            args.push(r.clone());
        }
        args.push(self.query);
        if let Some(j) = &self.json_fields {
            args.push("--json".to_string());
            args.push(j.clone());
        }
        if let Some(l) = self.limit {
            args.push("--limit".to_string());
            args.push(l.to_string());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::gh::run(&refs)
    }
}

// ---------------------------------------------------------------------------
// Builder: gh search repos
// ---------------------------------------------------------------------------

/// Start building a `gh search repos` command.
pub fn search_repos(query: &str) -> SearchRepos {
    SearchRepos {
        query: query.to_string(),
        json_fields: None,
        limit: None,
    }
}

/// Builder for `gh search repos`.
pub struct SearchRepos {
    query: String,
    json_fields: Option<String>,
    limit: Option<usize>,
}

impl SearchRepos {
    /// Select JSON output fields (comma-joined for `--json`).
    pub fn json(mut self, fields: &[&str]) -> Self {
        self.json_fields = Some(fields.join(","));
        self
    }

    /// Maximum number of results.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the command and return stdout.
    pub fn run(self) -> Result<String> {
        let mut args = vec!["search".to_string(), "repos".to_string(), self.query];
        if let Some(j) = &self.json_fields {
            args.push("--json".to_string());
            args.push(j.clone());
        }
        if let Some(l) = self.limit {
            args.push("--limit".to_string());
            args.push(l.to_string());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::gh::run(&refs)
    }
}

// ---------------------------------------------------------------------------
// Builder: gh api
// ---------------------------------------------------------------------------

/// Start building a `gh api` command.
pub fn api(endpoint: impl Into<String>) -> Api {
    Api {
        endpoint: endpoint.into(),
        jq: None,
    }
}

/// Builder for `gh api`.
pub struct Api {
    endpoint: String,
    jq: Option<String>,
}

impl Api {
    /// Apply a jq expression to the response.
    pub fn jq(mut self, expr: impl Into<String>) -> Self {
        self.jq = Some(expr.into());
        self
    }

    /// Execute the command and return stdout.
    pub fn run(self) -> Result<String> {
        let mut args = vec!["api".to_string(), self.endpoint];
        if let Some(j) = &self.jq {
            args.push("--jq".to_string());
            args.push(j.clone());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::gh::run(&refs)
    }
}
