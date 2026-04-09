# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

<!-- Describe your project here -->
<!-- This is a mixed Rust + Python project, typically using PyO3/maturin for Rust-Python interop -->

## Build & Test Commands

### Rust

```bash
cargo build                              # Build Rust code
cargo test                               # Rust tests
cargo clippy                             # Lint Rust
cargo fmt                                # Format Rust
```

### Python

```bash
uv sync                                  # Install/update Python dependencies
uv run maturin develop                   # Build and install Rust extension in dev mode
uv run pytest                            # Python tests
uv run ruff check .                      # Lint Python
uv run ruff format .                     # Format Python
```

## Pre-commit Checklist

**Run all before committing:**

```bash
# Rust
cargo fmt --all                                          # 1. Format Rust
cargo clippy --all-targets --all-features -- -D warnings # 2. Lint Rust

# Python
uv run ruff format .                                     # 3. Format Python
uv run ruff check . --fix                                # 4. Lint Python

# Tests
cargo test                                               # 5. Rust tests
uv run maturin develop                                   # 6. Build extension
uv run pytest                                            # 7. Python tests
```

## Project Structure

- `Cargo.toml` -- Rust project metadata and dependencies
- `pyproject.toml` -- Python project metadata and dependencies
- `uv.lock` -- locked Python dependency versions
- `src/` -- Rust source code (with PyO3 bindings)
- `python/` -- Python source code
- `tests/` -- Python tests

## Architecture

<!-- Describe your architecture here. Examples: -->
<!-- - Rust crate layout and module organization -->
<!-- - Python package structure -->
<!-- - PyO3 binding layer and exposed API -->
<!-- - Which logic lives in Rust vs Python -->

## Key Conventions

- **Rust edition:** 2024
- **Python version:** >= 3.12
- **Interop:** PyO3 with maturin for building
- **Error handling:** Rust errors are converted to Python exceptions via PyO3

## Git Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` (note the `!`) or add a `BREAKING CHANGE:` footer
