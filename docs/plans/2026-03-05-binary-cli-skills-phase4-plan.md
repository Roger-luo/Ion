# Binary CLI Skills Phase 4: Developer Experience

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make it easy for developers to create Ion-compatible binary CLI skills by enhancing the scaffolding and adding documentation.

**Architecture:** Enhance `ion new --bin` to generate a complete Rust CLI template with `skill` subcommand. Add a developer guide.

**Tech Stack:** Same as before — Rust, clap

---

## Task 1: Enhance `ion new --bin` scaffolding with skill subcommand

**Files:**
- Modify: `src/commands/new.rs`

Currently `ion new --bin` runs `cargo init --bin` and writes a generic SKILL.md. Enhance it to:

1. After `cargo init --bin`, write a `src/main.rs` template that includes:
   - A `skill` subcommand that outputs a SKILL.md to stdout
   - A placeholder default command
   - Uses clap for argument parsing

2. Update `Cargo.toml` to add clap dependency

3. Write a binary-specific SKILL.md template that includes `binary` metadata

### Template for `src/main.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "{name}", version, about = "{description}")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Output the SKILL.md for this tool (used by Ion during install)
    Skill,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Skill) => print_skill(),
        None => println!("Hello from {name}! Use --help for usage info."),
    }
}

fn print_skill() {
    print!(include_str!("../SKILL.md"));
}
```

### Binary-specific SKILL.md template:

```markdown
---
name: {name}
description: {description}
metadata:
  binary: {name}
  version: 0.1.0
---

# {title}

## Overview

Describe what this tool does. The agent invokes this via `ion run {name} [args]`.

## Usage

```bash
ion run {name} <command> [options]
```

## Commands

Describe available commands here.
```

### Changes to `run()`:
- When `bin` is true:
  1. Run `cargo init --bin` (existing)
  2. Append `clap = { version = "4", features = ["derive"] }` to the generated `Cargo.toml` under `[dependencies]`
  3. Write the `src/main.rs` template (overwrite the cargo init default)
  4. Write the binary SKILL.md template (instead of generic one)
  5. Print enhanced message: "Created binary skill project in {dir}. Run `cargo build` to compile."

**Tests:**
- Test that the binary SKILL.md template has correct metadata
- Test in tempdir that files are created correctly

---

## Task 2: Add developer guide for binary CLI skills

**Files:**
- Create: `docs/binary-skills-guide.md`

Write a concise developer guide covering:

1. **What are binary skills?** — CLI tools that work with Ion
2. **The `skill` subcommand convention** — Why it exists, what it should output
3. **SKILL.md metadata for binaries** — The `binary` field in metadata
4. **Publishing** — How to publish releases on GitHub for Ion compatibility
5. **Asset naming** — Naming conventions for release tarballs
6. **Testing locally** — How to test with `ion add --bin ./path`
7. **Example workflow** — From `ion new --bin` to `ion add --bin owner/repo`

Keep it under 200 lines. Focus on practical steps.

---

## Task 3: Integration test for `ion new --bin` scaffolding

**Files:**
- Modify or create: `tests/new_integration.rs`

Test that `ion new --bin` in a tempdir:
1. Creates `Cargo.toml` with clap dependency
2. Creates `src/main.rs` with skill subcommand
3. Creates `SKILL.md` with binary metadata
4. The generated project compiles (`cargo check` in the tempdir)
