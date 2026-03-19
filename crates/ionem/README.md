# ionem

Library for building [Ion](https://github.com/Roger-luo/Ion) binary skills with standard self-management commands.

Binary skills built for the Ion ecosystem implement a standard `self` subcommand group:

- `<binary> self skill` — print the embedded SKILL.md to stdout
- `<binary> self info` — show version, build target, and executable path
- `<binary> self check` — check if a newer version is available
- `<binary> self update` — download and install a newer version

`ionem` provides build-time SKILL.md preparation and runtime self-management via `SelfManager`.

## Quick start

### build.rs

For a plain SKILL.md (no templating):

```rust
fn main() {
    ionem::build::emit_target();
    ionem::build::copy_skill_md();
}
```

For a SKILL.md with variable substitution (`{version}`, `{name}`, `{description}` are auto-populated from Cargo metadata):

```rust
fn main() {
    ionem::build::emit_target();
    ionem::build::render_skill_md_vars(&[("author", "Alice")]);
}
```

For full control with your own template engine:

```rust
fn main() {
    ionem::build::emit_target();
    ionem::build::render_skill_md(|template| {
        my_engine::render(template, &context)
    });
}
```

### src/main.rs

```rust
use ionem::self_update::SelfManager;

const SKILL_MD: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));

let manager = SelfManager::new(
    "owner/my-tool",
    "my-tool",
    "v",
    env!("CARGO_PKG_VERSION"),
    env!("TARGET"),
);

// In your clap match:
// SelfCommands::Skill  => print!("{}", SKILL_MD),
// SelfCommands::Info   => manager.print_info(),
// SelfCommands::Check  => manager.print_check()?,
// SelfCommands::Update => manager.run_update(version.as_deref())?,
```

## Name

Latin *ionem* (accusative of *ion*) — ionizing a markdown skill into a binary executable.
