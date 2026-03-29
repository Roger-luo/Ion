---
title: "ionem::build"
description: "Build-script helpers for binary skills."
order: 999
---

Build-script helpers for binary skills.

These functions are designed to be called from `build.rs` to prepare
the SKILL.md and set up build-time environment variables.

# Plain SKILL.md

If your SKILL.md is a static file with no templating:

```rust,ignore
// build.rs
fn main() {
    ionem::build::emit_target();
    ionem::build::copy_skill_md();
}
```

# Template with variable substitution

If your SKILL.md uses `{version}`, `{name}`, `{description}` placeholders
(auto-populated from Cargo metadata) and/or custom variables:

```rust,ignore
// build.rs
fn main() {
    ionem::build::emit_target();
    ionem::build::render_skill_md_vars(&[("example_output", &generated_json)]);
}
```

# Custom template engine

For full control (e.g. using minijinja, tera, handlebars):

```rust,ignore
// build.rs
fn main() {
    ionem::build::emit_target();
    ionem::build::render_skill_md(|template| {
        my_engine::render(template, &context)
    });
}
```

# In main.rs

All approaches produce the same output. In your binary, use:

```rust,ignore
const SKILL_MD: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));
```

## emit_target

```rust
pub fn emit_target()
```

Emit the `TARGET` environment variable for the build.

This makes `env!("TARGET")` available in your binary, which is needed
by [`SelfManager`](crate::self_update::SelfManager) to report the build target.

Call this from `build.rs`.

---

## copy_skill_md

```rust
pub fn copy_skill_md()
```

Copy `SKILL.md` from the crate root to `OUT_DIR/SKILL.md` unchanged.

Sets up `cargo:rerun-if-changed` so the file is re-copied when modified.

# Panics

Panics if `SKILL.md` does not exist in the crate root or cannot be copied.

---

## copy_skill_md_from

```rust
pub fn copy_skill_md_from(path: &str)
```

Copy a SKILL.md from a custom path (relative to crate root) to `OUT_DIR/SKILL.md`.

Sets up `cargo:rerun-if-changed` so the file is re-copied when modified.

---

## render_skill_md

```rust
pub fn render_skill_md(render: impl FnOnce(&str) -> String)
```

Render `SKILL.md` from the crate root using a custom render function.

The function receives the raw template content and should return the
rendered output. Sets up `cargo:rerun-if-changed`.

# Example

```rust,ignore
ionem::build::render_skill_md(|content| {
    content.replace("{custom}", "value")
});
```

---

## render_skill_md_from

```rust
pub fn render_skill_md_from(path: &str, render: impl FnOnce(&str) -> String)
```

Render a SKILL.md template from a custom path using a custom render function.

Like [`render_skill_md`] but reads from an arbitrary path relative to the crate root.

---

## render_skill_md_vars

```rust
pub fn render_skill_md_vars(extra_vars: &[(&str, &str)])
```

Render `SKILL.md` with automatic variable substitution.

Automatically replaces these placeholders from Cargo package metadata:
- `{version}` — `CARGO_PKG_VERSION`
- `{name}` — `CARGO_PKG_NAME`
- `{description}` — `CARGO_PKG_DESCRIPTION`

Additional custom variables can be provided as `(key, value)` pairs.
Keys should not include braces — `("author", "Alice")` replaces `{author}`.

# Example

```rust,ignore
// SKILL.md contains: "Version: {version}, Author: {author}"
ionem::build::render_skill_md_vars(&[("author", "Alice")]);
// Produces: "Version: 0.1.0, Author: Alice"
```

---

## render_skill_md_vars_from

```rust
pub fn render_skill_md_vars_from(path: &str, extra_vars: &[(&str, &str)])
```

Render a SKILL.md template from a custom path with variable substitution.

Like [`render_skill_md_vars`] but reads from an arbitrary path.

---

## read_file

```rust
pub fn read_file(path: &str) -> String
```

Helper to read a file relative to the crate root.

Useful in `build.rs` when you need to read additional files
for template context (e.g. JSON examples).

