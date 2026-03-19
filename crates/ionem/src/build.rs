//! Build-script helpers for binary skills.
//!
//! These functions are designed to be called from `build.rs` to prepare
//! the SKILL.md and set up build-time environment variables.
//!
//! # Plain SKILL.md
//!
//! If your SKILL.md is a static file with no templating:
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     ionem::build::emit_target();
//!     ionem::build::copy_skill_md();
//! }
//! ```
//!
//! # Template with variable substitution
//!
//! If your SKILL.md uses `{version}`, `{name}`, `{description}` placeholders
//! (auto-populated from Cargo metadata) and/or custom variables:
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     ionem::build::emit_target();
//!     ionem::build::render_skill_md_vars(&[("example_output", &generated_json)]);
//! }
//! ```
//!
//! # Custom template engine
//!
//! For full control (e.g. using minijinja, tera, handlebars):
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     ionem::build::emit_target();
//!     ionem::build::render_skill_md(|template| {
//!         my_engine::render(template, &context)
//!     });
//! }
//! ```
//!
//! # In main.rs
//!
//! All approaches produce the same output. In your binary, use:
//!
//! ```rust,ignore
//! const SKILL_MD: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));
//! ```

use std::path::PathBuf;

/// Emit the `TARGET` environment variable for the build.
///
/// This makes `env!("TARGET")` available in your binary, which is needed
/// by [`SelfManager`](crate::self_update::SelfManager) to report the build target.
///
/// Call this from `build.rs`.
pub fn emit_target() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").unwrap())
}

/// Copy `SKILL.md` from the crate root to `OUT_DIR/SKILL.md` unchanged.
///
/// Sets up `cargo:rerun-if-changed` so the file is re-copied when modified.
///
/// # Panics
///
/// Panics if `SKILL.md` does not exist in the crate root or cannot be copied.
pub fn copy_skill_md() {
    copy_skill_md_from("SKILL.md");
}

/// Copy a SKILL.md from a custom path (relative to crate root) to `OUT_DIR/SKILL.md`.
///
/// Sets up `cargo:rerun-if-changed` so the file is re-copied when modified.
pub fn copy_skill_md_from(path: &str) {
    let src = manifest_dir().join(path);
    let dst = out_dir().join("SKILL.md");
    println!("cargo:rerun-if-changed={path}");
    std::fs::copy(&src, &dst).unwrap_or_else(|e| {
        panic!(
            "failed to copy {} to {}: {}",
            src.display(),
            dst.display(),
            e
        )
    });
}

/// Render `SKILL.md` from the crate root using a custom render function.
///
/// The function receives the raw template content and should return the
/// rendered output. Sets up `cargo:rerun-if-changed`.
///
/// # Example
///
/// ```rust,ignore
/// ionem::build::render_skill_md(|content| {
///     content.replace("{custom}", "value")
/// });
/// ```
pub fn render_skill_md(render: impl FnOnce(&str) -> String) {
    render_skill_md_from("SKILL.md", render);
}

/// Render a SKILL.md template from a custom path using a custom render function.
///
/// Like [`render_skill_md`] but reads from an arbitrary path relative to the crate root.
pub fn render_skill_md_from(path: &str, render: impl FnOnce(&str) -> String) {
    let src = manifest_dir().join(path);
    println!("cargo:rerun-if-changed={path}");
    let content = std::fs::read_to_string(&src)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", src.display(), e));
    let rendered = render(&content);
    let dst = out_dir().join("SKILL.md");
    std::fs::write(&dst, rendered)
        .unwrap_or_else(|e| panic!("failed to write {}: {}", dst.display(), e));
}

/// Render `SKILL.md` with automatic variable substitution.
///
/// Automatically replaces these placeholders from Cargo package metadata:
/// - `{version}` — `CARGO_PKG_VERSION`
/// - `{name}` — `CARGO_PKG_NAME`
/// - `{description}` — `CARGO_PKG_DESCRIPTION`
///
/// Additional custom variables can be provided as `(key, value)` pairs.
/// Keys should not include braces — `("author", "Alice")` replaces `{author}`.
///
/// # Example
///
/// ```rust,ignore
/// // SKILL.md contains: "Version: {version}, Author: {author}"
/// ionem::build::render_skill_md_vars(&[("author", "Alice")]);
/// // Produces: "Version: 0.1.0, Author: Alice"
/// ```
pub fn render_skill_md_vars(extra_vars: &[(&str, &str)]) {
    render_skill_md_vars_from("SKILL.md", extra_vars);
}

/// Render a SKILL.md template from a custom path with variable substitution.
///
/// Like [`render_skill_md_vars`] but reads from an arbitrary path.
pub fn render_skill_md_vars_from(path: &str, extra_vars: &[(&str, &str)]) {
    render_skill_md_from(path, |content| substitute(content, extra_vars));
}

fn substitute(content: &str, extra_vars: &[(&str, &str)]) -> String {
    let mut result = content.to_string();

    // Auto vars from Cargo
    let auto_vars = [
        ("version", "CARGO_PKG_VERSION"),
        ("name", "CARGO_PKG_NAME"),
        ("description", "CARGO_PKG_DESCRIPTION"),
    ];
    for (key, env_var) in auto_vars {
        if let Ok(val) = std::env::var(env_var) {
            result = result.replace(&format!("{{{key}}}"), &val);
        }
    }

    // User-provided vars
    for (key, val) in extra_vars {
        result = result.replace(&format!("{{{key}}}"), val);
    }

    result
}

/// Helper to read a file relative to the crate root.
///
/// Useful in `build.rs` when you need to read additional files
/// for template context (e.g. JSON examples).
pub fn read_file(path: &str) -> String {
    let full = manifest_dir().join(path);
    println!("cargo:rerun-if-changed={path}");
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", full.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_auto_vars() {
        // In test context, CARGO_PKG_* are set for ionem itself
        let input = "name: {name}, version: {version}";
        let result = substitute(input, &[]);
        assert!(result.contains("ionem"));
        assert!(!result.contains("{name}"));
    }

    #[test]
    fn test_substitute_custom_vars() {
        let input = "author: {author}, tool: {tool}";
        let result = substitute(input, &[("author", "Alice"), ("tool", "mytool")]);
        assert_eq!(result, "author: Alice, tool: mytool");
    }

    #[test]
    fn test_substitute_mixed() {
        let input = "{name} by {author}";
        let result = substitute(input, &[("author", "Bob")]);
        assert!(result.contains("ionem"));
        assert!(result.contains("Bob"));
        assert!(!result.contains("{author}"));
    }

    #[test]
    fn test_substitute_no_match_left_alone() {
        let input = "hello {unknown} world";
        let result = substitute(input, &[]);
        assert_eq!(result, "hello {unknown} world");
    }
}
