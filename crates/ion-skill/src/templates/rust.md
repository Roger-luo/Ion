# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

<!-- Describe your project here -->

## Build & Test Commands

```bash
cargo build                              # Dev build
cargo test                               # All tests
cargo test test_name                     # Single test by name
cargo clippy                             # Lint
cargo fmt                                # Format
```

## Pre-commit Checklist

**Run all three before committing** — CI checks these exact commands:

```bash
cargo fmt --all                                          # 1. Format
cargo clippy --all-targets --all-features -- -D warnings # 2. Lint (warnings are errors)
cargo test                                               # 3. Test
```

## Architecture

<!-- Describe your architecture here. Examples: -->
<!-- - Crate layout (binary vs library, workspace members) -->
<!-- - Key modules and their responsibilities -->
<!-- - Core data types and their relationships -->
<!-- - Important traits and their implementors -->

## Key Conventions

- **Error handling:** `anyhow::Result` for application code, `thiserror` for library errors
- **Rust edition:** 2024

## Git Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` (note the `!`) or add a `BREAKING CHANGE:` footer
