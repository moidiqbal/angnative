/**
 * bundle-third-party.mjs — third-party library spot-check bundle for QuickJS.
 *
 * Bundles an unmodified, already-locked third-party package (RxJS 7.8.2) into a
 * self-contained IIFE so the embedded runtime can execute it on the native
 * path. This is deliberately a *spot-check*, not a general compatibility
 * claim: RxJS is DOM-free. DOM-heavy UI libraries remain unsupported unchanged.
 *
 * Usage: node scripts/bundle-third-party.mjs
 * Output: dist/aot/rxjs-bundle.js
 */
import { build } from 'esbuild';
import path from 'node:path';
import { writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const pnpmStore = path.join(root, 'node_modules', '.pnpm', 'node_modules');
const rxjsEntry = require
  .resolve('rxjs', { paths: [pnpmStore] })
  .replaceAll('\\', '/');
const operatorsEntry = require
  .resolve('rxjs/operators', { paths: [pnpmStore] })
  .replaceAll('\\', '/');

const outDir = path.join(root, 'dist', 'aot');
const entry = path.join(outDir, 'rxjs-entry.js');
const outfile = path.join(outDir, 'rxjs-bundle.js');

await writeFile(
  entry,
  `import * as rxjs from ${JSON.stringify(rxjsEntry)};
import * as operators from ${JSON.stringify(operatorsEntry)};
globalThis.__thirdPartyRxjs = rxjs;
globalThis.__thirdPartyRxjsOperators = operators;
`,
  'utf8',
);

await build({
  entryPoints: [entry],
  outfile,
  bundle: true,
  format: 'iife',
  platform: 'browser',
  target: 'es2018',
  minify: false,
  logLevel: 'info',
});

console.log(`Wrote ${outfile}`);
