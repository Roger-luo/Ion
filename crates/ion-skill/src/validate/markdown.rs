use std::collections::BTreeSet;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub lang: String,
    pub code: String,
    pub start_line: usize,
}

pub fn extract_code_blocks(body: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<CodeBlock> = None;

    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_ascii_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                    CodeBlockKind::Indented => String::new(),
                };

                let start_line = body[..range.start].bytes().filter(|b| *b == b'\n').count() + 1;

                current = Some(CodeBlock {
                    lang,
                    code: String::new(),
                    start_line,
                });
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(ref mut block) = current {
                    block.code.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(ref mut block) = current {
                    block.code.push('\n');
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = current.take() {
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }

    blocks
}

pub fn extract_local_links(body: &str) -> Vec<String> {
    let mut links = Vec::new();

    for event in Parser::new(body) {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            let dest = dest_url.to_string();
            let lowered = dest.to_ascii_lowercase();
            let is_external = lowered.starts_with("http://")
                || lowered.starts_with("https://")
                || lowered.starts_with("mailto:")
                || lowered.starts_with("ftp://");

            if !dest.is_empty() && !is_external {
                links.push(dest);
            }
        }
    }

    links
}

pub fn extract_tool_mentions(body: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let re = Regex::new(r"\b(Bash|Read|Write|Edit|WebFetch|WebSearch|Agent|Glob|Grep)\b")
        .expect("tool mention regex must compile");

    for capture in re.captures_iter(body) {
        found.insert(capture[1].to_string());
    }

    found
}

#[cfg(test)]
mod tests {
    use super::{extract_code_blocks, extract_local_links, extract_tool_mentions};
    use std::collections::BTreeSet;

    #[test]
    fn extracts_fenced_blocks_with_language_and_line() {
        let body = "Intro\n```bash\necho hi\n```\n\n```python\nprint('x')\n```\n\n```rust\nfn main() {}\n```\n";

        let blocks = extract_code_blocks(body);

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].lang, "bash");
        assert_eq!(blocks[0].start_line, 2);
        assert!(blocks[1].code.contains("print('x')"));
        assert_eq!(blocks[2].lang, "rust");
    }

    #[test]
    fn extracts_markdown_links_and_filters_local_targets() {
        let body = "[Local](./references/setup.md) [Anchor](#usage) [Web](https://example.com) [Mail](mailto:test@example.com)";

        let links = extract_local_links(body);

        assert_eq!(
            links,
            vec!["./references/setup.md".to_string(), "#usage".to_string()]
        );
    }

    #[test]
    fn detects_tool_mentions_in_body_text() {
        let body = "Use Bash with Read and Write. Bash can call Grep.";

        let found = extract_tool_mentions(body);

        let expected: BTreeSet<String> = ["Bash", "Read", "Write", "Grep"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert_eq!(found, expected);
    }
}
