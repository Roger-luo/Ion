---
title: "ionem"
description: "Library for building Ion binary skills."
order: 100
---

*Version 0.2.0*

Library for building Ion binary skills.

Provides the standard self-management infrastructure that all Ion binary skills
are expected to implement: `self skill`, `self info`, `self check`, `self update`.

# Quick start

In `build.rs`:

```rust,ignore
fn main() {
    ionem::build::emit_target();
    ionem::build::copy_skill_md();  // or render_skill_md_vars / render_skill_md
}
```

In `src/main.rs`:

```rust,ignore
use ionem::self_update::SelfManager;

const SKILL_MD: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));

let manager = SelfManager::new(
    "owner/my-tool",
    "my-tool",
    "v",
    env!("CARGO_PKG_VERSION"),
    env!("TARGET"),
);
```

See [`build`] for SKILL.md preparation and [`self_update::SelfManager`] for the runtime API.

## Modules

| Module | Description |
|--------|-------------|
| [build](/docs/api-reference/ionem/build) | Build-script helpers for binary skills. |
| [error](/docs/api-reference/ionem/error) | Error types for binary skill operations. |
| [release](/docs/api-reference/ionem/release) | GitHub release fetching and platform detection for binary skills. |
| [self_update](/docs/api-reference/ionem/self-update) | Reusable self-management infrastructure for binary skills. |
| [shell](/docs/api-reference/ionem/shell) | CLI tool descriptors and wrappers for git, cargo, and gh. |

## Re-exports

- `pub use error::Error` as **Error**
- `pub use error::Result` as **Result**

