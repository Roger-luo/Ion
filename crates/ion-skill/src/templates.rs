//! Built-in AGENTS.md templates shipped with the ion binary.
//!
//! These templates provide offline project scaffolding for common language
//! ecosystems. When ion is updated, the embedded templates update too —
//! `ion agents update` will detect the new content via checksum comparison.

/// Prefix used to identify built-in templates in Ion.toml and the CLI.
pub const BUILTIN_PREFIX: &str = "builtin:";

/// List of available built-in template names.
pub const AVAILABLE: &[&str] = &[
    "generic",
    "rust",
    "python",
    "julia",
    "typescript",
    "rust+python",
];

/// Return the embedded template content for a given name, or `None` if unknown.
pub fn get(name: &str) -> Option<&'static str> {
    match name {
        // Fallback scaffold used when no language-specific template matches.
        "generic" => Some(include_str!("templates/generic.md")),
        "rust" => Some(include_str!("templates/rust.md")),
        "python" => Some(include_str!("templates/python.md")),
        "julia" => Some(include_str!("templates/julia.md")),
        "typescript" | "ts" => Some(include_str!("templates/typescript.md")),
        "rust+python" | "rust-python" => Some(include_str!("templates/rust-python.md")),
        _ => None,
    }
}

/// Parse a source string into a built-in template name if it matches.
///
/// Only the `builtin:` prefix is recognized (e.g. `builtin:rust`).
/// Bare names like `rust` are treated as remote sources to avoid
/// ambiguity with source aliases.
/// Returns `None` if the source doesn't refer to a known built-in template.
pub fn parse_builtin_name(source: &str) -> Option<&str> {
    let name = source.strip_prefix(BUILTIN_PREFIX)?;
    if get(name).is_some() {
        Some(name)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_known_templates() {
        assert!(get("rust").is_some());
        assert!(get("python").is_some());
        assert!(get("julia").is_some());
        assert!(get("typescript").is_some());
        assert!(get("ts").is_some()); // alias
        assert!(get("rust+python").is_some());
        assert!(get("rust-python").is_some()); // alias
    }

    #[test]
    fn get_unknown_returns_none() {
        assert!(get("go").is_none());
        assert!(get("").is_none());
    }

    #[test]
    fn parse_prefixed_name() {
        assert_eq!(parse_builtin_name("builtin:rust"), Some("rust"));
        assert_eq!(parse_builtin_name("builtin:python"), Some("python"));
        assert_eq!(parse_builtin_name("builtin:julia"), Some("julia"));
        assert_eq!(parse_builtin_name("builtin:typescript"), Some("typescript"));
        assert_eq!(parse_builtin_name("builtin:ts"), Some("ts"));
        assert_eq!(
            parse_builtin_name("builtin:rust+python"),
            Some("rust+python")
        );
    }

    #[test]
    fn bare_name_is_not_builtin() {
        assert_eq!(parse_builtin_name("rust"), None);
        assert_eq!(parse_builtin_name("python"), None);
    }

    #[test]
    fn parse_non_builtin_returns_none() {
        assert_eq!(parse_builtin_name("org/repo"), None);
        assert_eq!(parse_builtin_name("https://github.com/foo/bar"), None);
        assert_eq!(parse_builtin_name("builtin:go"), None);
    }

    #[test]
    fn templates_are_non_empty() {
        for name in AVAILABLE {
            let content = get(name).unwrap();
            assert!(!content.is_empty(), "template {name} should not be empty");
            assert!(
                content.contains("# AGENTS.md"),
                "template {name} should have AGENTS.md header"
            );
        }
    }
}
