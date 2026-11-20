/**
 * compile-aot.mjs — Angular AOT compilation script.
 *
 * Runs ngc AOT compilation on the app source, producing .ngfactory.js
 * and .ngstyle.js files that the Rust host can consume via the
 * AngNativeRenderer2.
 *
 * Usage:
 *   node scripts/compile-aot.mjs [tsconfig.aot.json]
 *
 * Default tsconfig: tsconfig.aot.json
 */

import { execSync } from 'child_process';
import { existsSync } from 'fs';

const AOT_TSCONFIG = process.argv[2] || 'tsconfig.aot.json';

if (!existsSync(AOT_TSCONFIG)) {
  console.error(`error: AOT tsconfig not found: ${AOT_TSCONFIG}`);
  console.error('  Create tsconfig.aot.json with Angular compiler options.');
  process.exit(1);
}

console.log(`[angnative aot] compiling with ${AOT_TSCONFIG}...`);

try {
  execSync(`npx ngc --project ${AOT_TSCONFIG}`, {
    stdio: 'inherit',
    cwd: process.cwd(),
  });
  console.log('[angnative aot] AOT compilation complete.');
} catch (err) {
  console.error(`error: AOT compilation failed: ${err.message}`);
  process.exit(1);
}
