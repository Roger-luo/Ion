---
title: "ionem::shell"
description: "CLI tool descriptors and error types for shell command wrappers (git, cargo, gh)."
order: 999
---

## Cli

Descriptor for a CLI tool — binary name and install hint.

Each module exposes a `CLI` constant of this type. The hint is attached to
every `NotFound` error so the user always knows how to install the tool.

```ignore
// Upfront check
gh::CLI.require()?;

// Or just run — hint is attached automatically on NotFound
gh::CLI.run_command(&mut gh::CLI.command().args(["repo", "view"]))?;
```

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `&'static str` | The binary name (e.g., `"git"`, `"gh"`, `"cargo"`). |
| `hint` | `&'static str` | Hint shown when the binary is not found (e.g., install URL). |

### Methods

#### `available`

```rust
pub fn available(&self) -> bool
```

Check if this CLI is available on PATH.

#### `require`

```rust
pub fn require(&self) -> Result<()>
```

Check availability, returning `Err(NotFound)` with the hint if missing.

#### `command`

```rust
pub fn command(&self) -> Command
```

Create a new [`Command`] for this CLI.

---

## CliError

Error from a CLI invocation.

### Variants

- **`NotFound { cli: String, hint: String }`** — The CLI binary was not found on PATH.
- **`Failed { cli: String, code: i32, stderr: String }`** — The command ran but exited with a non-zero status.
- **`Spawn { cli: String, source: io::Error }`** — I/O error spawning the process.
- **`InvalidUtf8 { cli: String }`** — Output was not valid UTF-8.

### Trait Implementations

- `Debug`
- `Error`
- `Display`

