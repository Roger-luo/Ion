use std::path::Path;

/// Known agent tool targets and their default skill directories.
const KNOWN_TARGETS: &[(&str, &str, &str)] = &[
    ("claude", ".claude", ".claude/skills"),
    ("cursor", ".cursor", ".cursor/skills"),
    ("windsurf", ".windsurf", ".windsurf/skills"),
];

/// Detect which known tool directories exist in the given project dir.
fn detect_targets(project_dir: &Path) -> Vec<(&'static str, &'static str)> {
    KNOWN_TARGETS
        .iter()
        .filter(|(_, dir, _)| project_dir.join(dir).is_dir())
        .map(|(name, _, path)| (*name, *path))
        .collect()
}

/// Parse a --target flag value. Accepts "name" (uses lookup) or "name:path".
fn parse_target_flag(flag: &str) -> anyhow::Result<(String, String)> {
    if let Some((name, path)) = flag.split_once(':') {
        if Path::new(path).is_absolute() {
            anyhow::bail!("Target paths must be relative to the project directory: {path}");
        }
        Ok((name.to_string(), path.to_string()))
    } else {
        let known = KNOWN_TARGETS.iter().find(|(n, _, _)| *n == flag);
        match known {
            Some((name, _, path)) => Ok((name.to_string(), path.to_string())),
            None => anyhow::bail!(
                "Unknown target '{flag}'. Known targets: claude, cursor, windsurf. \
                 Use 'name:path' for custom targets."
            ),
        }
    }
}

pub fn run(_targets: &[String], _force: bool) -> anyhow::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_target() {
        let (name, path) = parse_target_flag("claude").unwrap();
        assert_eq!(name, "claude");
        assert_eq!(path, ".claude/skills");
    }

    #[test]
    fn parse_custom_target() {
        let (name, path) = parse_target_flag("claude:.claude/commands/skills").unwrap();
        assert_eq!(name, "claude");
        assert_eq!(path, ".claude/commands/skills");
    }

    #[test]
    fn parse_unknown_target_is_error() {
        assert!(parse_target_flag("unknown").is_err());
    }

    #[test]
    fn parse_absolute_path_is_error() {
        assert!(parse_target_flag("foo:/absolute/path").is_err());
    }

    #[test]
    fn detect_targets_finds_existing_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        let detected = detect_targets(dir.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0], ("claude", ".claude/skills"));
    }

    #[test]
    fn detect_targets_empty_when_no_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_targets(dir.path());
        assert!(detected.is_empty());
    }
}
