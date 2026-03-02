use std::fmt;
use std::path::Path;

use crate::skill::SkillMetadata;

pub mod discovery;
pub mod markdown;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// How severe a validation finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    /// Numeric weight used for ordering (higher = more severe).
    fn weight(self) -> u8 {
        match self {
            Severity::Info => 0,
            Severity::Warning => 1,
            Severity::Error => 2,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.weight().cmp(&other.weight())
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Finding
// ---------------------------------------------------------------------------

/// A single validation finding produced by a checker.
#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub checker: String,
    pub message: String,
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// SkillChecker trait
// ---------------------------------------------------------------------------

/// Trait implemented by each validation checker.
pub trait SkillChecker {
    /// A short, human-readable name for this checker.
    fn name(&self) -> &str;

    /// Run the check and return zero or more findings.
    fn check(&self, skill_dir: &Path, meta: &SkillMetadata, body: &str) -> Vec<Finding>;
}

// ---------------------------------------------------------------------------
// Runner & helpers
// ---------------------------------------------------------------------------

/// Run every registered checker and return all findings sorted by severity
/// descending (errors first).
pub fn run_all_checkers(
    skill_dir: &Path,
    meta: &SkillMetadata,
    body: &str,
) -> Vec<Finding> {
    let checkers: Vec<Box<dyn SkillChecker>> = vec![];

    let mut findings: Vec<Finding> = checkers
        .iter()
        .flat_map(|c| c.check(skill_dir, meta, body))
        .collect();

    // Sort by severity descending (Error first, then Warning, then Info).
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}

/// Returns `true` if any finding has `Severity::Error`.
pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Error)
}

/// Returns `true` if any finding has `Severity::Warning`.
pub fn has_warnings(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Warning)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to build a minimal `SkillMetadata` for tests.
    fn dummy_meta() -> SkillMetadata {
        SkillMetadata {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            license: None,
            compatibility: None,
            metadata: Some(HashMap::new()),
            allowed_tools: None,
        }
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Error > Severity::Info);
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "ERROR");
        assert_eq!(Severity::Warning.to_string(), "WARN");
        assert_eq!(Severity::Info.to_string(), "INFO");
    }

    #[test]
    fn has_errors_detects_errors() {
        let findings = vec![
            Finding {
                severity: Severity::Warning,
                checker: "test".into(),
                message: "warn".into(),
                detail: None,
            },
            Finding {
                severity: Severity::Error,
                checker: "test".into(),
                message: "err".into(),
                detail: None,
            },
        ];
        assert!(has_errors(&findings));
    }

    #[test]
    fn has_errors_returns_false_when_none() {
        let findings = vec![
            Finding {
                severity: Severity::Info,
                checker: "test".into(),
                message: "info".into(),
                detail: None,
            },
            Finding {
                severity: Severity::Warning,
                checker: "test".into(),
                message: "warn".into(),
                detail: None,
            },
        ];
        assert!(!has_errors(&findings));
    }

    #[test]
    fn run_all_checkers_returns_empty_with_no_checkers() {
        let meta = dummy_meta();
        let findings = run_all_checkers(Path::new("/tmp"), &meta, "body");
        assert!(findings.is_empty());
    }
}
