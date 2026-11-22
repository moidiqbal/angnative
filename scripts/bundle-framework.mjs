/**
 * bundle-framework.mjs — Phase 2/3 Angular feature bundles for QuickJS.
 *
 * Produces self-contained IIFEs exposing the real @angular/router and
 * @angular/animations APIs on globalThis for in-runtime verification.
 *
 * Usage: node scripts/bundle-framework.mjs
 * Output: dist/aot/router-bundle.js, dist/aot/animations-bundle.js
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
const httpEntry = require
  .resolve('@angular/common/http', { paths: [pnpmStore] })
  .replaceAll('\\', '/');
const coreEntry = require
  .resolve('@angular/core', { paths: [pnpmStore] })
  .replaceAll('\\', '/');


const outDir = path.join(root, 'dist', 'aot');

const routerAppEntry = path
  .join(root, 'dist', 'aot-router', 'router', 'main.router.js')
  .replaceAll('\\', '/');

const features = [
  { name: 'router', pkg: '@angular/router', global: '__ngRouter' },
  { name: 'animations', pkg: '@angular/animations', global: '__ngAnimations' },
  {
    name: 'router-app',
    source: `import ${JSON.stringify(routerAppEntry)};`,
  },
  {
    name: 'http',
    global: '__ngHttp',
    source: `import * as http from ${JSON.stringify(httpEntry)};
import { createEnvironmentInjector, runInInjectionContext, provideZonelessChangeDetection, DestroyRef, Injector } from ${JSON.stringify(coreEntry)};
globalThis.__ngHttp = Object.assign({}, http, {
  createEnvironmentInjector, runInInjectionContext, provideZonelessChangeDetection, DestroyRef, Injector
});`,
  },
];

for (const feature of features) {
  const entry = path.join(outDir, `${feature.name}-entry.js`);
  const outfile = path.join(outDir, `${feature.name}-bundle.js`);

  const body = feature.source
    ?? `import * as ng from ${JSON.stringify(feature.pkg)};
globalThis.${feature.global} = ng;`;
  await writeFile(
    entry,
    `import ${JSON.stringify(compilerEntry)};\n${body}\n`,
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
    // Secondary entry points are not resolvable from the generated dist dir.
    logLevel: 'info',
  });
  console.log(`Wrote ${outfile}`);
}
