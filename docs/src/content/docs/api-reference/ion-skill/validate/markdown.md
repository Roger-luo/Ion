---
title: "ion-skill::validate::markdown"
description: ""
order: 999
---

## CodeBlock

### Fields

| Name | Type | Description |
|------|------|-------------|
| `lang` | `String` |  |
| `code` | `String` |  |
| `start_line` | `usize` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

---

## extract_code_blocks

```rust
pub fn extract_code_blocks(body: &str) -> Vec<CodeBlock>
```

---

## extract_local_links

```rust
pub fn extract_local_links(body: &str) -> Vec<String>
```

---

## extract_tool_mentions

```rust
pub fn extract_tool_mentions(body: &str) -> BTreeSet<String>
```

