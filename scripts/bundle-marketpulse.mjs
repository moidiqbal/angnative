/**
 * bundle-marketpulse.mjs — self-contained QuickJS bundle for the MarketPulse
 * AOT demo. Mirrors bundle-quickjs.mjs but reads dist/aot-marketpulse output.
 *
 * Usage:
 *   pnpm compile:aot:marketpulse && pnpm bundle:marketpulse
 *   ./gradlew assembleDebug
 *
 * The Gradle build stages this as `assets/market-pulse-bundle.js` alongside the
 * counter's stable `assets/angnative-bundle.js`; the Android shell selects the
 * MarketPulse asset at runtime. No Gradle property is needed.
 */
import { build } from 'esbuild';
import path from 'node:path';
import { writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const pnpmStore = path.join(root, 'node_modules', '.pnpm', 'node_modules');
const compilerEntry = require
  .resolve('@angular/compiler', { paths: [pnpmStore] })
  .replaceAll('\\', '/');
// @angular/common is transitive in this workspace; resolve its http subpath
// from pnpm's virtual store so esbuild can bundle the HttpClient provider.
const httpEntry = require
  .resolve('@angular/common/http', { paths: [pnpmStore] })
  .replaceAll('\\', '/');
const entry = path.join(root, 'dist', 'aot-marketpulse', 'marketpulse', 'main.marketpulse.js');
const outfile = path.join(root, 'dist', 'aot', 'market-pulse-bundle.js');
const quickjsEntry = path.join(root, 'dist', 'aot', 'market-pulse-entry.js');

// The @angnative/common primitives ship as decorated JS and need Angular's
// JIT compiler registered before bootstrap (same as the shipped counter app).
await writeFile(
  quickjsEntry,
  `import ${JSON.stringify(compilerEntry)};\nimport ${JSON.stringify(entry)};\n`,
  'utf8',
);

await build({
  entryPoints: [quickjsEntry],
  outfile,
  bundle: true,
  format: 'iife',
  platform: 'browser',
  target: 'es2018',
  minify: true,
  // RxJS 6's tslib banner contains blank lines with trailing spaces; dropping
  // legal comments keeps the generated asset clean for `git diff --check`.
  legalComments: 'none',
  alias: { '@angular/common/http': httpEntry },
  define: { ngDevMode: 'false' },
  logLevel: 'info',
});

console.log(`Wrote ${outfile}`);
