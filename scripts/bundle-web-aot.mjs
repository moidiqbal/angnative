#!/usr/bin/env node
/**
 * bundle-web-aot.mjs — bundle the ngc AOT web entry for the browser demo.
 *
 * Prerequisite: `pnpm compile:aot:web` (real ngc output in dist/aot/web).
 *
 * Unlike the old JIT demo, this entry has no @angular/compiler import. The
 * build fails if the metafile shows a JIT compiler package was pulled in.
 */
import { build } from 'esbuild';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';
import { angularLinkerEsbuildPlugin } from './angular-linker-plugin.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const entry = join(root, 'dist', 'aot', 'web', 'main.web.js');
const outDir = join(root, 'dist', 'web-demo');
const outfile = join(outDir, 'app.js');
const metafilePath = join(outDir, 'app.meta.json');

if (!existsSync(entry)) {
  console.error(`error: AOT web entry not found: ${entry}`);
  console.error('  Run: pnpm compile:aot:web');
  process.exit(1);
}

// Guard against accidentally bundling the old non-AOT main.web.js output.
for (const name of ['app.component.js', 'counter.component.js']) {
  const path = join(root, 'dist', 'aot', 'web', name);
  if (!existsSync(path) || !/ɵɵdefineComponent/.test(readFileSync(path, 'utf-8'))) {
    console.error(`error: missing Ivy definition in AOT web output: ${path}`);
    console.error('  Run: pnpm compile:aot:web');
    process.exit(1);
  }
}

mkdirSync(outDir, { recursive: true });

const result = await build({
  entryPoints: [entry],
  outfile,
  bundle: true,
  format: 'esm',
  platform: 'browser',
  target: 'es2022',
  minify: true,
  sourcemap: true,
  metafile: true,
  logLevel: 'info',
  // Angular packages ship partial declarations; link them instead of loading JIT.
  plugins: [angularLinkerEsbuildPlugin()],
  // The workspace link for @angnative/web lives under apps/demo-app, while the
  // compiled entry lives under dist/aot/web. Alias explicitly so the AOT
  // bundle resolves the same package the source entry used.
  alias: {
    '@angnative/web': join(root, 'packages', 'web', 'dist', 'index.js'),
  },
  define: {
    ngDevMode: 'false',
    ngJitMode: 'false',
    ngI18nClosureMode: 'false',
    ngRuntimeStrictMode: 'true',
    ngDevModeAndNotTest: 'false',
    ngBrowserAnimation: 'false',
  },
});

writeFileSync(metafilePath, JSON.stringify(result.metafile, null, 2), 'utf-8');

const inputs = Object.keys(result.metafile.inputs).join('\n');
const jitPackages = ['@angular/compiler', '@angular/platform-browser-dynamic'];
const pulledJit = jitPackages.filter((pkg) => inputs.includes(pkg));
if (pulledJit.length > 0) {
  console.error(`error: AOT web bundle pulled in JIT packages: ${pulledJit.join(', ')}`);
  console.error('  Inputs containing a JIT package:\n' + inputs.split('\n').filter((line) => jitPackages.some((pkg) => line.includes(pkg))).join('\n'));
  process.exit(1);
}

console.log(`[angnative web] AOT bundle written to ${outfile}`);
console.log(`[angnative web] JIT compiler packages absent from metafile inputs`);
