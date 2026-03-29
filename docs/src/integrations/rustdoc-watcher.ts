/**
 * Astro integration that watches Rust source files during `astro dev`
 * and re-runs the rustdoc → markdown generation pipeline on change.
 *
 * Astro's built-in content collection watcher picks up the regenerated
 * .md files and triggers HMR automatically.
 */

import type { AstroIntegration } from "astro";
import { execSync } from "node:child_process";
import { resolve } from "node:path";
import { watch } from "node:fs";

export default function rustdocWatcher(): AstroIntegration {
  return {
    name: "rustdoc-watcher",
    hooks: {
      "astro:server:setup"({ logger }) {
        const docsDir = resolve(import.meta.dirname!, "../..");
        const projectRoot = resolve(docsDir, "..");
        const cratesDir = resolve(projectRoot, "crates");
        const scriptPath = resolve(docsDir, "scripts/generate-api-docs.ts");

        let debounceTimer: ReturnType<typeof setTimeout> | null = null;
        const DEBOUNCE_MS = 1500;

        function regenerate() {
          logger.info("Rust source changed — regenerating API docs...");
          try {
            execSync(`npx tsx ${scriptPath}`, {
              cwd: docsDir,
              stdio: "inherit",
            });
            logger.info("API docs regenerated.");
          } catch (e) {
            logger.error("API docs generation failed.");
          }
        }

        function onFileChange(filename: string | null) {
          if (!filename?.endsWith(".rs")) return;
          if (debounceTimer) clearTimeout(debounceTimer);
          debounceTimer = setTimeout(regenerate, DEBOUNCE_MS);
        }

        // Watch crates/*/src/ recursively
        const watcher = watch(cratesDir, { recursive: true }, (_event, filename) => {
          onFileChange(filename as string | null);
        });

        logger.info(`Watching ${cratesDir} for Rust source changes`);

        // Clean up on process exit
        process.on("exit", () => watcher.close());
      },
    },
  };
}
