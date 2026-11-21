/**
 * bundle-forms.mjs — Phase 1 of Angular feature compatibility in QuickJS.
 *
 * Produces a self-contained IIFE that exposes the real @angular/forms API on
 * globalThis.__ngForms so it can be exercised inside the embedded runtime.
 *
 * Usage: node scripts/bundle-forms.mjs
 * Output: dist/aot/forms-bundle.js
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
const outDir = path.join(root, 'dist', 'aot');
const entry = path.join(outDir, 'forms-entry.js');
const outfile = path.join(outDir, 'forms-bundle.js');

await writeFile(
  entry,
  `import ${JSON.stringify(compilerEntry)};
import {
  FormControl, FormGroup, FormArray, Validators, AbstractControl
} from '@angular/forms';
globalThis.__ngForms = {
  FormControl, FormGroup, FormArray, Validators, AbstractControl
};
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
  define: { ngDevMode: 'false' },
  logLevel: 'info',
});

console.log(`Wrote ${outfile}`);
