---
title: "ionem::shell::cargo"
description: "Cargo CLI wrappers."
order: 999
---

Cargo CLI wrappers.

Use [`require()`] to verify `cargo` is installed, then call methods on the
returned [`Cargo`] handle:

```ignore
let cargo = cargo::require()?;
let proj = cargo.project(&manifest_path);
let meta = proj.metadata()?;
proj.build_release()?;
let output = proj.run("my-bin", &["arg1"])?;
```

## Cargo

A validated handle proving the `cargo` CLI is available.

Obtained via [`require()`]. Context constructors and standalone
operations live here.

### Methods

#### `project`

```rust
pub fn project<'a>(&self, manifest_path: &'a Path) -> Project<'a>
```

Create a [`Project`] context bound to the given `Cargo.toml` path.

#### `init`

```rust
pub fn init(&self, path: &Path, name: &str) -> Result<()>
```

Run `cargo init` to create a new project.

---

## CargoMetadata

Parsed output of `cargo metadata`.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `String` |  |
| `version` | `String` |  |
| `binary_targets` | `Vec<String>` | Names of all binary targets. |
| `manifest_path` | `String` | Path to the manifest file. |

### Trait Implementations

- `Debug`

---

## Project

A Cargo project context that binds a manifest path, so you can call
multiple operations without repeating the path.

*…and private fields*

### Methods

#### `metadata`

```rust
pub fn metadata(&self) -> Result<CargoMetadata>
```

Run `cargo metadata --no-deps` and parse the result.

#### `build_release`

```rust
pub fn build_release(&self) -> Result<()>
```

Run `cargo build --release`.

#### `build_release_bin`

```rust
pub fn build_release_bin(&self, bin: &str) -> Result<()>
```

Run `cargo build --release` for a specific binary target.

#### `run`

```rust
pub fn run(&self, bin: &str, args: &[&str]) -> Result<String>
```

Run `cargo run -q` with the given arguments. Returns stdout.

#### `run_interactive`

```rust
pub fn run_interactive(&self, bin: &str, args: &[&str]) -> Result<()>
```

Run `cargo run` inheriting stdio (for interactive use).

---

## require

```rust
pub fn require() -> Result<Cargo>
```

Verify `cargo` is installed and return a handle to run commands.

---

## raw_metadata

```rust
pub fn raw_metadata(manifest_path: &Path) -> Result<String>
```

Run `cargo metadata --no-deps` and return the raw JSON string.
Useful when callers need to inspect all workspace packages directly.

---

## metadata

```rust
pub fn metadata(manifest_path: &Path) -> Result<CargoMetadata>
```

Run `cargo metadata --no-deps` and parse the result.
`manifest_path` should point to the Cargo.toml file.

---

## build_release

```rust
pub fn build_release(manifest_path: &Path) -> Result<()>
```

Run `cargo build --release` for a project.

---

## build_release_bin

```rust
pub fn build_release_bin(manifest_path: &Path, bin: &str) -> Result<()>
```

Run `cargo build --release` for a specific binary target.

---

## run

```rust
pub fn run(manifest_path: &Path, bin: &str, args: &[&str]) -> Result<String>
```

Run `cargo run -q` with the given arguments. Returns stdout.
`manifest_path` should point to the Cargo.toml file.
`bin` is the binary target name.
`args` are passed after `--` to the binary.

---

## run_interactive

```rust
pub fn run_interactive(manifest_path: &Path, bin: &str, args: &[&str]) -> Result<()>
```

Run `cargo run` inheriting stdio (for interactive use).

---

## init

```rust
pub fn init(path: &Path, name: &str) -> Result<()>
```

Run `cargo init` to create a new project.

---

## project

```rust
pub fn project(manifest_path: &Path) -> Project<'_>
```

Create a [`Project`] context bound to the given `Cargo.toml` path.

