import { defineConfig } from 'astro/config';
import remarkCallouts from './src/plugins/remark-callouts.mjs';
import remarkSidenotes from './src/plugins/remark-sidenotes.mjs';
import rustdocWatcher from './src/integrations/rustdoc-watcher.ts';

export default defineConfig({
  site: 'https://ion.dev',
  integrations: [rustdocWatcher()],
  markdown: {
    remarkPlugins: [remarkCallouts, remarkSidenotes],
    shikiConfig: {
      themes: {
        light: 'vitesse-light',
        dark: 'vitesse-dark',
      },
    },
  },
});
