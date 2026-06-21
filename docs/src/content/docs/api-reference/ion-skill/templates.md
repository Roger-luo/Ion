---
title: "ion-skill::templates"
description: "Built-in AGENTS.md templates shipped with the ion binary."
order: 999
---

Built-in AGENTS.md templates shipped with the ion binary.

These templates provide offline project scaffolding for common language
ecosystems. When ion is updated, the embedded templates update too —
`ion agents update` will detect the new content via checksum comparison.

## get

```rust
pub fn get(name: &str) -> Option<&'static str>
```

Return the embedded template content for a given name, or `None` if unknown.

---

## parse_builtin_name

```rust
pub fn parse_builtin_name(source: &str) -> Option<&str>
```

Parse a source string into a built-in template name if it matches.

Only the `builtin:` prefix is recognized (e.g. `builtin:rust`).
Bare names like `rust` are treated as remote sources to avoid
ambiguity with source aliases.
Returns `None` if the source doesn't refer to a known built-in template.

