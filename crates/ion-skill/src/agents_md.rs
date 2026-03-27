//! AGENTS.md management: fetch from URL, write managed sections, update.
//!
//! An AGENTS.md file may contain an org-standard "managed" section fetched
//! from a remote URL, wrapped in HTML-style comment markers:
//!
//! ```markdown
//! <!-- ion:managed:begin -->
//! ... content fetched from the configured URL ...
//! <!-- ion:managed:end -->
//!
//! ## Project-Specific Notes
//!
//! Add project-specific guidance below the managed section.
//! ```
//!
//! The managed section is identified by the `<!-- ion:managed:begin -->` /
//! `<!-- ion:managed:end -->` markers.  Everything outside those markers is
//! preserved unchanged on every `ion agents update` call.

use std::path::Path;

use crate::{Error, Result};

/// Delimiter placed at the start of the managed section.
pub const MANAGED_BEGIN: &str = "<!-- ion:managed:begin -->";
/// Delimiter placed at the end of the managed section.
pub const MANAGED_END: &str = "<!-- ion:managed:end -->";

/// Result of writing or updating an AGENTS.md file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteResult {
    /// A new AGENTS.md file was created.
    Created,
    /// The managed section of an existing AGENTS.md was updated.
    Updated,
    /// The existing file was left unchanged (content was already up-to-date).
    Unchanged,
}

/// Fetch the raw text content from a URL.
///
/// Accepts `https://` and `http://` URLs.  Also accepts GitHub shorthand
/// `owner/repo/path/to/file` which is expanded to the corresponding
/// `raw.githubusercontent.com` URL using the default branch (`HEAD`).
pub fn fetch_content(url: &str) -> Result<String> {
    let resolved = resolve_url(url);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&resolved)
        .header("User-Agent", "ion-skill-manager")
        .send()
        .map_err(|e| Error::Http(format!("Failed to fetch AGENTS.md: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!(
            "Server returned {} for {}",
            resp.status(),
            resolved
        )));
    }
    resp.text()
        .map_err(|e| Error::Http(format!("Failed to read response body: {e}")))
}

/// Resolve a URL-or-shorthand to a full URL.
///
/// * `https://…` and `http://…` are returned as-is.
/// * Everything else is treated as a GitHub shorthand
///   `owner/repo/path/to/file` and expanded to the corresponding
///   `raw.githubusercontent.com/owner/repo/HEAD/path/to/file` URL.
pub fn resolve_url(url: &str) -> String {
    if url.starts_with("https://") || url.starts_with("http://") {
        return url.to_string();
    }
    // GitHub shorthand: owner/repo/path/to/file
    // We need at least 3 segments: owner / repo / file
    let parts: Vec<&str> = url.splitn(3, '/').collect();
    if parts.len() == 3 {
        let (owner, repo, file_path) = (parts[0], parts[1], parts[2]);
        return format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{file_path}");
    }
    // Fall back to treating it as a URL fragment (will likely fail at fetch time)
    url.to_string()
}

/// Write a new AGENTS.md file from remote content.
///
/// The file is created with:
/// 1. The managed section (wrapped in begin/end markers).
/// 2. A trailing project-specific placeholder section.
///
/// Returns an error if the file already exists and `force` is `false`.
pub fn write_new(fetched: &str, path: &Path, force: bool) -> Result<WriteResult> {
    if path.exists() && !force {
        return Err(Error::Manifest(format!(
            "AGENTS.md already exists at {}.  Use --force to overwrite.",
            path.display()
        )));
    }
    let content = build_full_content(fetched);
    std::fs::write(path, content).map_err(Error::Io)?;
    Ok(WriteResult::Created)
}

/// Update the managed section inside an existing AGENTS.md file.
///
/// * If the file does not yet exist it is created (same as `write_new`).
/// * If the file exists but has no managed markers the managed section is
///   prepended so that existing project content is preserved below it.
/// * If managed markers are present their inner content is replaced with the
///   newly fetched text.
///
/// Returns `WriteResult::Unchanged` when the managed section content is
/// identical to `fetched` (no disk write is performed in that case).
pub fn update_managed(fetched: &str, path: &Path) -> Result<WriteResult> {
    if !path.exists() {
        let content = build_full_content(fetched);
        std::fs::write(path, content).map_err(Error::Io)?;
        return Ok(WriteResult::Created);
    }

    let existing = std::fs::read_to_string(path).map_err(Error::Io)?;

    if let Some(updated) = replace_managed_section(&existing, fetched) {
        if updated == existing {
            return Ok(WriteResult::Unchanged);
        }
        std::fs::write(path, &updated).map_err(Error::Io)?;
        Ok(WriteResult::Updated)
    } else {
        // No managed markers — prepend the managed section, preserve everything else
        let content = format!("{}\n\n{}", build_managed_block(fetched), existing);
        std::fs::write(path, &content).map_err(Error::Io)?;
        Ok(WriteResult::Updated)
    }
}

/// Extract the content currently inside the managed section markers, if any.
pub fn extract_managed_content(text: &str) -> Option<&str> {
    let begin = text.find(MANAGED_BEGIN)?;
    let after_begin = &text[begin + MANAGED_BEGIN.len()..];
    let end = after_begin.find(MANAGED_END)?;
    Some(after_begin[..end].trim())
}

// ── private helpers ──────────────────────────────────────────────────────────

fn build_managed_block(content: &str) -> String {
    format!("{MANAGED_BEGIN}\n{content}\n{MANAGED_END}")
}

fn build_full_content(fetched: &str) -> String {
    format!(
        "{}\n\n## Project-Specific Notes\n\n\
         <!-- Add project-specific guidance below this line. -->\n",
        build_managed_block(fetched)
    )
}

/// Replace the content inside managed markers in `text` with `new_content`.
///
/// Returns `None` if no markers were found.
fn replace_managed_section(text: &str, new_content: &str) -> Option<String> {
    let begin_pos = text.find(MANAGED_BEGIN)?;
    let after_begin = begin_pos + MANAGED_BEGIN.len();
    let end_pos = text[after_begin..].find(MANAGED_END)? + after_begin;
    let after_end = end_pos + MANAGED_END.len();

    let result = format!(
        "{}{}\n{}\n{}{}",
        &text[..begin_pos],
        MANAGED_BEGIN,
        new_content,
        MANAGED_END,
        &text[after_end..]
    );
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_passes_through_https() {
        assert_eq!(
            resolve_url("https://example.com/AGENTS.md"),
            "https://example.com/AGENTS.md"
        );
    }

    #[test]
    fn resolve_url_passes_through_http() {
        assert_eq!(
            resolve_url("http://example.com/AGENTS.md"),
            "http://example.com/AGENTS.md"
        );
    }

    #[test]
    fn resolve_url_expands_github_shorthand() {
        assert_eq!(
            resolve_url("myorg/myrepo/AGENTS.md"),
            "https://raw.githubusercontent.com/myorg/myrepo/HEAD/AGENTS.md"
        );
    }

    #[test]
    fn resolve_url_expands_github_shorthand_with_subpath() {
        assert_eq!(
            resolve_url("myorg/myrepo/docs/AGENTS.md"),
            "https://raw.githubusercontent.com/myorg/myrepo/HEAD/docs/AGENTS.md"
        );
    }

    #[test]
    fn write_new_creates_file_with_markers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let content = "# Org Standard\n\nDo things.";
        write_new(content, &path, false).unwrap();

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains(MANAGED_BEGIN));
        assert!(on_disk.contains(MANAGED_END));
        assert!(on_disk.contains("# Org Standard"));
        assert!(on_disk.contains("Project-Specific Notes"));
    }

    #[test]
    fn write_new_fails_if_file_exists_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "existing").unwrap();
        let err = write_new("content", &path, false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn write_new_overwrites_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "existing").unwrap();
        let result = write_new("new content", &path, true).unwrap();
        assert_eq!(result, WriteResult::Created);
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("new content"));
    }

    #[test]
    fn update_managed_creates_file_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let result = update_managed("org content", &path).unwrap();
        assert_eq!(result, WriteResult::Created);
        assert!(path.exists());
    }

    #[test]
    fn update_managed_updates_existing_managed_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let initial =
            format!("{MANAGED_BEGIN}\nold org content\n{MANAGED_END}\n\n## Project Notes\n\nmine.");
        std::fs::write(&path, &initial).unwrap();

        let result = update_managed("new org content", &path).unwrap();
        assert_eq!(result, WriteResult::Updated);

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("new org content"));
        assert!(!on_disk.contains("old org content"));
        assert!(on_disk.contains("## Project Notes"));
        assert!(on_disk.contains("mine."));
    }

    #[test]
    fn update_managed_returns_unchanged_when_content_identical() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let content = "org content";
        // Build the file as update_managed would
        let initial = format!("{MANAGED_BEGIN}\n{content}\n{MANAGED_END}\n\n");
        std::fs::write(&path, &initial).unwrap();

        let result = update_managed(content, &path).unwrap();
        assert_eq!(result, WriteResult::Unchanged);
    }

    #[test]
    fn update_managed_prepends_when_no_markers_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# My Existing Notes\n\nStuff.").unwrap();

        let result = update_managed("org content", &path).unwrap();
        assert_eq!(result, WriteResult::Updated);

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains(MANAGED_BEGIN));
        assert!(on_disk.contains("org content"));
        assert!(on_disk.contains("# My Existing Notes"));
        // Managed section should come before existing content
        let begin_pos = on_disk.find(MANAGED_BEGIN).unwrap();
        let existing_pos = on_disk.find("# My Existing Notes").unwrap();
        assert!(begin_pos < existing_pos);
    }

    #[test]
    fn extract_managed_content_finds_content() {
        let text = format!("{MANAGED_BEGIN}\nsome text\n{MANAGED_END}");
        assert_eq!(extract_managed_content(&text), Some("some text"));
    }

    #[test]
    fn extract_managed_content_returns_none_when_absent() {
        let text = "# No markers here";
        assert_eq!(extract_managed_content(text), None);
    }
}
