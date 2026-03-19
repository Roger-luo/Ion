# ionem

Library for building [Ion](https://github.com/Roger-luo/Ion) binary skills with standard self-management commands.

Binary skills built for the Ion ecosystem implement a standard `self` subcommand group:

- `<binary> self skill` — print the embedded SKILL.md to stdout
- `<binary> self info` — show version, build target, and executable path
- `<binary> self check` — check if a newer version is available
- `<binary> self update` — download and install a newer version

`ionem` provides `SelfManager` which implements the core logic for `info`, `check`, and `update`.

## Quick start

```rust
use ionem::self_update::SelfManager;

let manager = SelfManager::new(
    "owner/my-tool",          // GitHub repo
    "my-tool",                // binary name in release assets
    "v",                      // tag prefix (e.g. "v1.0.0")
    env!("CARGO_PKG_VERSION"),
    env!("TARGET"),           // set via build.rs
);

// In your clap match:
// SelfCommands::Skill  => print!(include_str!("../SKILL.md")),
// SelfCommands::Info   => manager.print_info(),
// SelfCommands::Check  => manager.print_check()?,
// SelfCommands::Update => manager.run_update(version.as_deref())?,
```

## Name

Latin *ionem* (accusative of *ion*) — ionizing a markdown skill into a binary executable.
