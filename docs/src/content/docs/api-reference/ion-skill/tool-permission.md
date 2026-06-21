---
title: "ion-skill::tool_permission"
description: ""
order: 999
---

## ToolRequest

A tool request parsed from a `<request-tool>` element in a skill body.

A request may optionally include a `<scope>` to narrow the permission.
For example:

```xml
<request-tool>
  <tool>Bash</tool>
  <scope>cargo tree:*</scope>
</request-tool>
```

### Fields

| Name | Type | Description |
|------|------|-------------|
| `tool` | `String` | The tool name, e.g. `"Bash"`. |
| `scope` | `Option<String>` | An optional scope pattern that narrows the request, e.g. `"cargo tree:*"`. |

### Methods

#### `new`

```rust
pub fn new(tool: impl Into<String>) -> Self
```

Create a new unscoped tool request.

#### `scoped`

```rust
pub fn scoped(tool: impl Into<String>, scope: impl Into<String>) -> Self
```

Create a new scoped tool request.

#### `approval_label`

```rust
pub fn approval_label(&self) -> String
```

The label shown in the approval prompt.

Scoped requests display as `Tool(scope)` (e.g. `Bash(cargo tree:*)`);
unscoped requests display the bare tool name (e.g. `Bash`).

#### `grant`

```rust
pub fn grant(&self) -> ToolPermission
```

Produce the [`ToolPermission`] that should be recorded in the session
when the user approves this request.

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`
- `Display`

---

## ToolPermission

A permission granted to a session for a specific tool.

### Variants

- **`Allow(String)`** — Unrestricted access to the named tool.
- **`AllowScoped { tool: String, scope: String }`** — Access to the named tool, restricted to commands matching the scope pattern.

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`
- `Display`

---

## parse_tool_requests

```rust
pub fn parse_tool_requests(body: &str) -> Vec<ToolRequest>
```

Parse all `<request-tool>` elements from a skill body.

Each element must contain a `<tool>` child; an optional `<scope>` child
narrows the request.  Returns an empty vec when no elements are found.

