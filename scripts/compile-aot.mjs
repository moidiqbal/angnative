/**
 * compile-aot.mjs — REAL Angular AOT (ngc) compilation.
 *
 * Why this exists: `npx ngc` in this workspace resolves to a shadowing 0.1.0
 * binary that only emits plain JS (no Ivy component definitions). The genuine
 * Angular compiler-cli lives inside pnpm's virtual store. We resolve it from
 * `@angular/compiler-cli/package.json`'s `bin.ngc` field instead, so the output
 * under dist/aot contains ɵɵdefineComponent / template functions.
 *
 * Usage:
 *   node scripts/compile-aot.mjs [tsconfig.aot.json]
 */

import { execFileSync } from 'child_process';
import { existsSync, readFileSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join, resolve } from 'path';

const require = createRequire(import.meta.url);
const AOT_TSCONFIG = process.argv[2] || 'tsconfig.aot.json';

if (!existsSync(AOT_TSCONFIG)) {
  console.error(`error: AOT tsconfig not found: ${AOT_TSCONFIG}`);
  process.exit(1);
}

function resolveAngularNgc() {
  const pkgJson = require.resolve('@angular/compiler-cli/package.json');
  const pkg = JSON.parse(readFileSync(pkgJson, 'utf-8'));
  const bin = pkg.bin?.ngc || './bundles/src/bin/ngc.js';
  return resolve(dirname(pkgJson), bin);
}

const ngc = resolveAngularNgc();
if (!existsSync(ngc)) {
  console.error(`error: Angular ngc not found at: ${ngc}`);
  process.exit(1);
}

console.log(`[angnative aot] Angular compiler-cli: ${ngc}`);
console.log(`[angnative aot] compiling with ${AOT_TSCONFIG}...`);
try {
  execFileSync(process.execPath, [ngc, '--project', AOT_TSCONFIG], {
    stdio: 'inherit',
    cwd: process.cwd(),
  });
} catch (err) {
  console.error(`error: AOT compilation failed: ${err.message}`);
  process.exit(1);
}

// Verify real AOT output — at least one Ivy component definition.
const outDir = join(process.cwd(), 'dist', 'aot');
if (!existsSync(outDir)) {
  console.error('error: AOT output directory missing');
  process.exit(1);
}
let ivyFound = false;
for (const file of ['app.component.js', 'counter.component.js']) {
  const path = join(outDir, file);
  if (existsSync(path) && /defineComponent|ɵɵdefineComponent/.test(readFileSync(path, 'utf-8'))) {
    ivyFound = true;
    break;
  }
}
if (!ivyFound) {
  console.error('error: AOT output has no Ivy component definitions — compiler was not real ngc');
  process.exit(1);
}
console.log('[angnative aot] AOT compilation complete (Ivy definitions verified).');
