/**
 * tsc7.mjs — run the TypeScript 7 compiler explicitly.
 *
 * The root `typescript` dependency stays on 6.0.3 because Angular 22.3.0's
 * compiler-cli only supports TS 6.x. TypeScript 7 is installed under the
 * `typescript7` alias. Both packages expose a `tsc` bin, so relying on pnpm's
 * bin de-duplication would be platform-dependent; this wrapper resolves the
 * alias package and invokes its declared bin deterministically.
 *
 * Usage:
 *   node scripts/tsc7.mjs -b
 */

import { execFileSync } from 'child_process';
import { readFileSync } from 'fs';
import { createRequire } from 'module';
import { dirname, resolve } from 'path';

const require = createRequire(import.meta.url);
const pkgJson = require.resolve('typescript7/package.json');
const pkg = JSON.parse(readFileSync(pkgJson, 'utf-8'));
const bin = typeof pkg.bin === 'string' ? pkg.bin : pkg.bin?.tsc;

if (!bin) {
  console.error('error: typescript7 alias does not declare a tsc bin');
  process.exit(1);
}

const tsc = resolve(dirname(pkgJson), bin);
try {
  execFileSync(process.execPath, [tsc, ...process.argv.slice(2)], { stdio: 'inherit' });
} catch (err) {
  process.exit(err.status ?? 1);
}
