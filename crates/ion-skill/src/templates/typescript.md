# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

<!-- Describe your project here -->

## Build & Test Commands

```bash
npm install                              # Install dependencies
npm run build                            # Build (typically tsc or bundler)
npm test                                 # All tests
npm test -- --testNamePattern="name"     # Single test by name (Jest)
npm run lint                             # Lint (ESLint)
npm run format                           # Format (Prettier)
npx tsc --noEmit                         # Type-check without emitting
```

If the project uses `pnpm` or `yarn`, substitute the package manager
(`pnpm install`, `yarn install`, etc.). If it uses Bun, use `bun install`,
`bun test`, `bun run build`.

## Pre-commit Checklist

**Run all four before committing:**

```bash
npm run format                                           # 1. Format
npm run lint                                             # 2. Lint
npx tsc --noEmit                                         # 3. Type-check
npm test                                                 # 4. Test
```

## Project Structure

- `package.json` -- project metadata, dependencies, and scripts
- `package-lock.json` / `pnpm-lock.yaml` / `yarn.lock` -- locked versions (committed to git)
- `tsconfig.json` -- TypeScript compiler configuration
- `src/` -- source code (`.ts` / `.tsx`)
- `tests/` or `__tests__/` -- test files (or `*.test.ts` co-located with source)
- `dist/` or `build/` -- compiled output (gitignored)

## Architecture

<!-- Describe your architecture here. Examples: -->
<!-- - Module organization and entry points (main, exports field) -->
<!-- - Key types/interfaces and their relationships -->
<!-- - Public API surface vs internal modules -->
<!-- - Runtime targets (Node, browser, both) and bundler setup -->

## Key Conventions

- **TypeScript:** `strict` mode enabled in `tsconfig.json`; avoid `any` — prefer `unknown` and narrow with type guards
- **Module system:** Prefer ES modules (`import`/`export`); set `"type": "module"` in `package.json` for Node
- **Type definitions:** Co-locate types with implementation; expose public types from the package entry point
- **Async:** Prefer `async`/`await` over raw Promise chains; handle rejections explicitly
- **Imports:** Use absolute or path-aliased imports for cross-module references; relative imports within a module

## Git Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` (note the `!`) or add a `BREAKING CHANGE:` footer
