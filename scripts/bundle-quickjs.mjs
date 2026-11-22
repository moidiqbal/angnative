/**
 * bundle-quickjs.mjs — produce a self-contained JS bundle embeddable in QuickJS.
 *
 * Prerequisite: `pnpm compile:aot` (generates dist/aot/main.js via ngc).
 *
 * Why this exists:
 *  - `packages/cli/src/bundle.ts` emits ESM with `packages: 'external'`, so
 *    `dist/aot/bundle.js` cannot run inside QuickJS (no module loader).
 *  - esbuild's default ES2022 output uses syntax QuickJS-ng 0.5.1 cannot parse.
 *    target 'es2018' keeps the output parseable.
 *
 * Output: dist/aot/quickjs-bundle.js (IIFE, no imports, Angular included).
 *
 * The bundle includes @angular/compiler explicitly so the decorated
 * @angnative/common primitives can be JIT-compiled. It is verified to boot a
 * real Angular app in QuickJS:
 *   pnpm compile:aot && pnpm bundle:quickjs
 *   cargo test -p angnative-runtime-bridge -- --ignored
 * (acceptance test angular_bootstrap_runs_in_quickjs, with generated bundles).
 */
import { build } from 'esbuild';
import path from 'node:path';
import { writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
// pnpm's virtual store is not on the default resolution path when esbuild is
// imported as a library (only `pnpm exec esbuild` adds it via NODE_PATH).
const pnpmStore = path.join(root, 'node_modules', '.pnpm', 'node_modules');
const compilerEntry = require
  .resolve('@angular/compiler', { paths: [pnpmStore] })
  .replaceAll('\\', '/');
const entry = path.join(root, 'dist', 'aot', 'main.js');
const outfile = path.join(root, 'dist', 'aot', 'quickjs-bundle.js');
const quickjsEntry = path.join(root, 'dist', 'aot', 'quickjs-entry.js');

// The @angnative/common primitives ship as decorated (non-Ivy) JS, so Angular
// needs its JIT compiler registered before bootstrap. A real file on disk (as
// opposed to esbuild stdin) lets esbuild's pnpm-aware resolver find it.
await writeFile(
  quickjsEntry,
  `import ${JSON.stringify(compilerEntry)};\nimport './main.js';\n`,
  'utf8',
);

await build({
  entryPoints: [quickjsEntry],
  outfile,
  bundle: true,
  format: 'iife',
  platform: 'browser',
  target: 'es2018',
  minify: false,
  define: { ngDevMode: 'false' },
  logLevel: 'info',
});

console.log(`Wrote ${outfile}`);
