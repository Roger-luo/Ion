# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

<!-- Describe the site or application, its users, and its main purpose. -->

## Build & Test Commands

Follow the committed lockfile when choosing npm, pnpm, yarn, or Bun. The
examples below use npm. Only invoke scripts that are defined in `package.json`.
Run `astro check` through the selected package manager's Astro command.

```bash
npm install                 # Install dependencies
npm run dev                 # Start the development server
npm run build               # Create a production build
npm run preview             # Preview the production build
npm run astro -- check      # Check Astro and TypeScript diagnostics
npm test                    # Optional: run tests when this script exists
npm run lint                # Optional: lint when this script exists
npm run format              # Optional: format when this script exists
```

## Pre-commit Checklist

Run the commands that the repository defines before committing:

```bash
npm run format              # Format, when defined
npm run lint                # Lint, when defined
npm run astro -- check      # Astro and TypeScript diagnostics
npm test                    # Tests, when defined
npm run build               # Production build
```

## Project Structure

- `package.json` -- dependencies and the authoritative list of scripts
- The committed lockfile -- package manager selection and locked dependencies
- `astro.config.*` -- Astro integrations and build configuration
- `src/pages/` -- file-based routes and endpoint files
- `src/layouts/` -- shared page shells
- `src/components/` -- reusable Astro and UI components
- `src/content/` -- content collection entries
- `src/content.config.ts` or `src/content/config.ts` -- collection schemas and loaders, depending on the Astro version
- `public/` -- files copied unchanged and served from the site root

## Architecture

<!-- Describe the rendering strategy and boundaries between pages, layouts, and components. -->
<!-- Identify content collections, their schemas, and where content is queried. -->
<!-- Record dynamic routes, endpoints, middleware, and important data flows. -->
<!-- Note which components require browser hydration and why. -->

## Key Conventions

- **Server first:** Render HTML on the server by default. Add browser JavaScript only when interaction requires it.
- **Hydration:** Use an explicit `client:*` directive for interactive components and choose the least eager directive that meets the UX requirement.
- **Astro components:** Put component scripts, imports, and data loading in the fenced `.astro` frontmatter; keep the template focused on rendering.
- **Props:** Define typed `Props` for `.astro` components and destructure values from `Astro.props`.
- **Routing:** Treat files under `src/pages/` as routes. Keep route names and dynamic parameters aligned with the intended URL shape.
- **Content:** Define schemas in the project's content config and use Astro's content collection APIs to query and render entries instead of reading content files directly.
- **Images:** Prefer Astro's image APIs for source-controlled images that need optimization. Put passthrough assets in `public/` and reference them with root-relative URLs.
- **Secrets:** Read private values only in server-side code. Never expose secrets through client-hydrated components or public environment variables.
- **TypeScript:** Preserve strict typing and avoid `any`; use `unknown` with explicit narrowing when input is not trusted.

## Git Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` or add a `BREAKING CHANGE:` footer.
