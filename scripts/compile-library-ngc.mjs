/**
 * compile-library-ngc.mjs — build Angular libraries with real ngc partial
 * compilation, so emitted declarations carry the Ivy metadata that downstream
 * AOT compilations need.
 *
 * Why this exists: `tsc -b` emits plain declarations for @angnative/common,
 * which makes ngc report NG2012 when demo components import those components.
 * The genuine compiler-cli lives inside pnpm's virtual store, so resolve its
 * `bin.ngc` field instead of using the shadowing `ngc` binary.
 *
 * Usage:
 *   node scripts/compile-library-ngc.mjs [tsconfig]
 */

import { execFileSync } from 'child_process';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join, resolve } from 'path';

const require = createRequire(import.meta.url);
const LIBRARY_TSCONFIG = process.argv[2] || 'packages/common/tsconfig.ngc.json';
const EXTRA_NGC_ARGS = process.argv.slice(3);

if (!existsSync(LIBRARY_TSCONFIG)) {
  console.error(`error: library tsconfig not found: ${LIBRARY_TSCONFIG}`);
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

console.log(`[angnative library ngc] Angular compiler-cli: ${ngc}`);
console.log(`[angnative library ngc] compiling with ${LIBRARY_TSCONFIG}...`);
try {
  execFileSync(process.execPath, [ngc, '--project', LIBRARY_TSCONFIG, ...EXTRA_NGC_ARGS], {
    stdio: 'inherit',
    cwd: process.cwd(),
  });
} catch (err) {
  console.error(`error: library ngc compilation failed: ${err.message}`);
  process.exit(1);
}

const tsconfig = JSON.parse(readFileSync(LIBRARY_TSCONFIG, 'utf-8'));
const outDir = resolve(dirname(resolve(LIBRARY_TSCONFIG)), tsconfig.compilerOptions?.outDir ?? 'dist');

function hasPartialMetadata(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (hasPartialMetadata(path)) return true;
    } else if (entry.name.endsWith('.js') || entry.name.endsWith('.d.ts')) {
      if (/ɵɵngDeclareComponent|ɵɵdefineComponent/.test(readFileSync(path, 'utf-8'))) return true;
    }
  }
  return false;
}

if (!existsSync(outDir) || !hasPartialMetadata(outDir)) {
  console.error('error: library output has no Ivy metadata — ngc partial compilation required');
  process.exit(1);
}
console.log('[angnative library ngc] partial compilation complete (Ivy metadata verified).');
