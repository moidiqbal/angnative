import { defineConfig } from 'vitest/config';
import {
  angularLinkerInlineDeps,
  angularLinkerVitePlugin,
} from './scripts/angular-linker-plugin.mjs';

export default defineConfig({
  plugins: [angularLinkerVitePlugin()],
  test: {
    hookTimeout: 60000,
    testTimeout: 60000,
    server: {
      deps: {
        inline: angularLinkerInlineDeps,
      },
    },
  },
});
