import { configDefaults, defineConfig } from 'vitest/config';
import {
  angularLinkerInlineDeps,
  angularLinkerVitePlugin,
} from './scripts/angular-linker-plugin.mjs';

export default defineConfig({
  plugins: [angularLinkerVitePlugin()],
  test: {
    // tsc -b emits compiled copies of *.spec.ts into package dist folders.
    // Vitest 5 includes them by default; run source specs only.
    exclude: [...configDefaults.exclude, '**/dist/**'],
    hookTimeout: 60000,
    testTimeout: 60000,
    server: {
      deps: {
        inline: angularLinkerInlineDeps,
      },
    },
  },
});
