use std::path::Path;

use crate::skill::SkillMetadata;

use super::markdown::extract_code_blocks;
use super::{Finding, Severity, SkillChecker};

pub struct TreeSitterCodeBlockChecker;

fn language_for_tag(tag: &str) -> Option<tree_sitter::Language> {
    match tag {
        "bash" | "sh" => Some(tree_sitter_bash::LANGUAGE.into()),
        "python" | "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "rust" | "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        _ => None,
    }
}

impl SkillChecker for TreeSitterCodeBlockChecker {
    fn name(&self) -> &str {
        "codeblock-tree-sitter"
    }

    fn check(&self, _skill_dir: &Path, _meta: &SkillMetadata, body: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        for block in extract_code_blocks(body) {
            let Some(language) = language_for_tag(&block.lang) else {
                continue;
            };

            let mut parser = tree_sitter::Parser::new();
            if parser.set_language(&language).is_err() {
                continue;
            }

            let Some(tree) = parser.parse(&block.code, None) else {
                findings.push(Finding {
                    severity: Severity::Warning,
                    checker: self.name().to_string(),
                    message: format!(
                        "Parser did not return a tree for '{}' block at line {}",
                        block.lang, block.start_line
                    ),
                    detail: None,
                });
                continue;
            };

            if tree.root_node().has_error() {
                // Use Warning instead of Error: skill code blocks often contain
                // pseudo-code, placeholder templates (e.g. <branch-name>), or
                // abbreviated examples that aren't meant to be valid syntax.
                findings.push(Finding {
                    severity: Severity::Warning,
                    checker: self.name().to_string(),
                    message: format!(
                        "Possibly invalid {} code block starting at line {}",
                        block.lang, block.start_line
                    ),
                    detail: None,
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::TreeSitterCodeBlockChecker;
    use crate::skill::SkillMetadata;
    use crate::validate::{Severity, SkillChecker};

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
    fn invalid_python_block_is_warning() {
        let checker = TreeSitterCodeBlockChecker;
        let body = "```python\ndef foo(:\n```";

        let findings = checker.check(std::path::Path::new("."), &dummy_meta(), body);

        assert!(findings.iter().any(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn invalid_bash_block_is_warning() {
        let checker = TreeSitterCodeBlockChecker;
        let body = "```bash\nif [[\n```";

        let findings = checker.check(std::path::Path::new("."), &dummy_meta(), body);

        assert!(findings.iter().any(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn invalid_rust_block_is_warning() {
        let checker = TreeSitterCodeBlockChecker;
        let body = "```rust\nfn main( {\n```";

        let findings = checker.check(std::path::Path::new("."), &dummy_meta(), body);

        assert!(findings.iter().any(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn unknown_language_block_is_ignored_or_info() {
        let checker = TreeSitterCodeBlockChecker;
        let body = "```haskell\nmain = putStrLn \"x\"\n```";

        let findings = checker.check(std::path::Path::new("."), &dummy_meta(), body);

        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }
}
