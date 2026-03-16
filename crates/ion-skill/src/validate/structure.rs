use std::path::Path;

use crate::skill::SkillMetadata;

use super::markdown::{extract_local_links, extract_tool_mentions};
use super::{Finding, Severity, SkillChecker};

pub struct ReferenceIntegrityChecker;
pub struct ToolDeclarationConsistencyChecker;

impl SkillChecker for ReferenceIntegrityChecker {
    fn name(&self) -> &str {
        "reference-integrity"
    }

    fn check(&self, skill_dir: &Path, _meta: &SkillMetadata, body: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        for link in extract_local_links(body) {
            let target = link.split('#').next().unwrap_or_default().trim();
            if target.is_empty() {
                continue;
            }

            let ref_path = Path::new(target);
            let has_parent = ref_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));

            if ref_path.is_absolute() || has_parent {
                findings.push(Finding {
                    severity: Severity::Warning,
                    checker: self.name().to_string(),
                    message: format!("Path traversal or absolute path reference: {link}"),
                    detail: None,
                });
                continue;
            }

            if !skill_dir.join(ref_path).exists() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    checker: self.name().to_string(),
                    message: format!("Missing referenced local file: {target}"),
                    detail: None,
                });
            }
        }

        findings
    }
}

impl SkillChecker for ToolDeclarationConsistencyChecker {
    fn name(&self) -> &str {
        "tool-declaration"
    }

    fn check(&self, _skill_dir: &Path, meta: &SkillMetadata, body: &str) -> Vec<Finding> {
        let mentioned = extract_tool_mentions(body);
        if mentioned.is_empty() {
            return vec![];
        }

        let Some(allowed) = meta.allowed_tools.as_deref() else {
            return vec![Finding {
                severity: Severity::Warning,
                checker: self.name().to_string(),
                message: "Body references tools but allowed-tools is not declared".to_string(),
                detail: Some(format!(
                    "Mentioned tools: {}",
                    mentioned.iter().cloned().collect::<Vec<_>>().join(", ")
                )),
            }];
        };

        let declared: std::collections::BTreeSet<String> = allowed
            .split([',', ' '])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();

        let missing: Vec<String> = mentioned.difference(&declared).cloned().collect();

        if missing.is_empty() {
            return vec![];
        }

        vec![Finding {
            severity: Severity::Warning,
            checker: self.name().to_string(),
            message: "Tools referenced in body are missing from allowed-tools".to_string(),
            detail: Some(format!("Undeclared tools: {}", missing.join(", "))),
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ReferenceIntegrityChecker, ToolDeclarationConsistencyChecker};
    use crate::skill::SkillMetadata;
    use crate::validate::{Severity, SkillChecker};

    fn meta_with_allowed_tools(value: Option<&str>) -> SkillMetadata {
        SkillMetadata {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            license: None,
            compatibility: None,
            metadata: Some(HashMap::new()),
            allowed_tools: value.map(ToString::to_string),
        }
    }

    #[test]
    fn reports_missing_referenced_local_files() {
        let root = tempfile::tempdir().unwrap();
        let checker = ReferenceIntegrityChecker;
        let body = "See [Setup](./references/setup.md)";

        let findings = checker.check(root.path(), &meta_with_allowed_tools(None), body);

        assert!(
            findings
                .iter()
                .any(|f| { f.severity == Severity::Warning && f.checker == "reference-integrity" })
        );
    }

    #[test]
    fn reports_path_traversal_references() {
        let root = tempfile::tempdir().unwrap();
        let checker = ReferenceIntegrityChecker;
        let body = "Bad ref [secret](../secrets.txt)";

        let findings = checker.check(root.path(), &meta_with_allowed_tools(None), body);

        assert!(
            findings.iter().any(|f| {
                f.severity == Severity::Warning && f.message.contains("Path traversal")
            })
        );
    }

    #[test]
    fn reports_missing_allowed_tools_when_tools_are_mentioned() {
        let root = tempfile::tempdir().unwrap();
        let checker = ToolDeclarationConsistencyChecker;
        let body = "Use Bash to inspect files";

        let findings = checker.check(root.path(), &meta_with_allowed_tools(None), body);

        assert!(findings.iter().any(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn accepts_existing_relative_references() {
        let root = tempfile::tempdir().unwrap();
        let ref_file = root.path().join("references/setup.md");
        std::fs::create_dir_all(ref_file.parent().unwrap()).unwrap();
        std::fs::write(&ref_file, "# Setup\n").unwrap();

        let refs_checker = ReferenceIntegrityChecker;
        let tools_checker = ToolDeclarationConsistencyChecker;
        let body = "See [Setup](./references/setup.md). Use Bash.";
        let meta = meta_with_allowed_tools(Some("Bash, Read"));

        let mut findings = refs_checker.check(root.path(), &meta, body);
        findings.extend(tools_checker.check(root.path(), &meta, body));

        assert!(findings.is_empty());
    }
}
