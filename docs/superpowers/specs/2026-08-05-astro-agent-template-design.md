# Astro Built-in Agent Template Design

## Goal

Add an offline `builtin:astro` AGENTS.md template and make `ion init` select it automatically for Astro projects.

## Template

Ship `crates/ion-skill/src/templates/astro.md` as a dedicated template rather than reusing or composing the TypeScript template. It will remain package-manager neutral and cover core Astro with TypeScript:

- standard install, development, build, preview, test, lint, format, and type-check commands;
- Astro's `astro check` validation command;
- the conventional `src/pages`, `src/layouts`, `src/components`, `src/content`, and `public` structure;
- `.astro` component conventions and frontmatter;
- content collections;
- server-first rendering and explicit client hydration;
- project-specific placeholders for architecture and project description.

The template will avoid integration-specific guidance for UI frameworks, CSS systems, and deployment adapters.

## Registration and Detection

Register `astro` in the canonical built-in template list and map it to the embedded Markdown file. `builtin:astro` will work anywhere existing built-ins work.

`ion init` will identify Astro from any root-level `astro.config.mjs`, `astro.config.js`, `astro.config.ts`, or `astro.config.cjs` file. Astro detection will take precedence over generic TypeScript detection. Existing Rust, Python, Rust-plus-Python, and Julia precedence will remain unchanged, preserving current behavior in mixed-language repositories.

## User-facing Documentation

Update the workflow guide's project-detection wording to include Astro and TypeScript so the documented list reflects actual behavior.

## Error Handling

No new error path is required. Unknown built-in names will continue to use the existing error behavior, and missing project markers will continue to fall back to the generic template.

## Testing

Use test-driven development:

1. Add unit expectations for Astro template lookup, prefixed-name parsing, non-empty content, and Astro-over-TypeScript detection.
2. Add an integration test that creates an Astro marker, runs `ion init --json`, and verifies that `builtin:astro` is selected.
3. Run formatting, linting, and the full test suite before completion.

