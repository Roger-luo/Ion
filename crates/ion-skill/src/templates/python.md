# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

<!-- Describe your project here -->

## Build & Test Commands

```bash
uv sync                                  # Install/update dependencies
uv run pytest                            # All tests
uv run pytest tests/test_name.py         # Single test file
uv run pytest -k test_name               # Single test by name
uv run ruff check .                      # Lint
uv run ruff format .                     # Format
```

## Pre-commit Checklist

**Run all three before committing:**

```bash
uv run ruff format .                     # 1. Format
uv run ruff check . --fix                # 2. Lint (auto-fix where possible)
uv run pytest                            # 3. Test
```

## Project Structure

This project uses [uv](https://docs.astral.sh/uv/) for dependency and environment management.

- `pyproject.toml` -- project metadata and dependencies
- `uv.lock` -- locked dependency versions (committed to git)
- `src/` -- source code
- `tests/` -- test files

## Architecture

<!-- Describe your architecture here. Examples: -->
<!-- - Package layout and module organization -->
<!-- - Key classes/modules and their responsibilities -->
<!-- - Core data types and their relationships -->
<!-- - Important protocols/ABCs and their implementors -->

## Key Conventions

- **Python version:** >= 3.12
- **Type hints:** Use type annotations for all public functions
- **Dependency management:** Use `uv add <package>` to add dependencies, `uv add --dev <package>` for dev dependencies

## Git Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` (note the `!`) or add a `BREAKING CHANGE:` footer
