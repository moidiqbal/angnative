/**
 * check-size-budget.mjs — CI gate for the JS size budget.
 *
 * Reads the production bundle + metafile produced by `bundle:fixtures`,
 * compares the gzipped size against budgets.json, and fails if:
 *   - the gzipped payload exceeds the hello-world budget (<= 60 KB)
 *   - the metafile is missing
 *   - the bundle contains @angular/compiler or zone.js
 *
 * Exit codes:
 *   0 — within budget
 *   1 — budget exceeded or forbidden package found
 */

import { readFileSync, existsSync } from 'fs';
import { createGzip } from 'zlib';
import { join } from 'path';

const REPO_ROOT = process.cwd();

const BUDGETS_PATH = join(REPO_ROOT, 'packages', 'cli', 'src', 'budgets.json');
const BUNDLE_PATH = join(
  REPO_ROOT,
  'packages',
  'testing',
  'fixtures',
  'out',
  'bundle.js'
);
const METAFILE_PATH = join(
  REPO_ROOT,
  'packages',
  'testing',
  'fixtures',
  'out',
  'bundle.meta.json'
);

/** Load budgets.json — the single source of truth for size limits. */
function loadBudgets() {
  if (!existsSync(BUDGETS_PATH)) {
    console.error(`error: budgets.json not found at ${BUDGETS_PATH}`);
    process.exit(1);
  }
  return JSON.parse(readFileSync(BUDGETS_PATH, 'utf-8'));
}

/** Gzip a file and return the compressed byte count. */
function gzipSize(filePath) {
  const input = readFileSync(filePath);
  return new Promise((resolve, reject) => {
    const gzip = createGzip();
    const chunks = [];
    gzip.on('data', (c) => chunks.push(c));
    gzip.on('end', () => resolve(Buffer.concat(chunks).length));
    gzip.on('error', reject);
    gzip.end(input);
  });
}

/** Scan metafile inputs for forbidden packages. */
function findForbidden(metafile, forbidden) {
  const inputs = (metafile.inputs || {});
  const inputPaths = Object.keys(inputs);
  return forbidden.filter((fp) => inputPaths.some((p) => p.includes(fp)));
}

async function main() {
  const budgets = loadBudgets();
  const budgetBytes = budgets.helloWorldGzipBytes;
  const forbiddenPackages = budgets.forbiddenPackages || [
    '@angular/compiler',
    'zone.js',
  ];

  // 1. Metafile must exist
  if (!existsSync(METAFILE_PATH)) {
    console.error(`error: bundle metafile missing at ${METAFILE_PATH}`);
    console.error('  Run: pnpm run bundle:fixtures');
    process.exit(1);
  }

  // 2. Bundle must exist
  if (!existsSync(BUNDLE_PATH)) {
    console.error(`error: bundle not found at ${BUNDLE_PATH}`);
    console.error('  Run: pnpm run bundle:fixtures');
    process.exit(1);
  }

  const metafile = JSON.parse(readFileSync(METAFILE_PATH, 'utf-8'));
  const gzipBytes = await gzipSize(BUNDLE_PATH);

  // 3. Check forbidden packages
  const forbiddenFound = findForbidden(metafile, forbiddenPackages);
  if (forbiddenFound.length > 0) {
    console.error(
      `error: bundle contains forbidden packages: ${forbiddenFound.join(', ')}`
    );
    console.error(
      '  @angular/compiler and zone.js must never reach a device bundle.'
    );
    process.exit(1);
  }

  // 4. Check gzip budget
  const withinBudget = gzipBytes <= budgetBytes;

  console.log(
    `JS payload: ${gzipBytes} B gzip (budget ${budgetBytes} B / ${
      budgetBytes / 1024
    } KB)`
  );
  console.log('@angular/compiler and zone.js: 0 B in the bundle.');

  // Per-package breakdown
  const inputs = metafile.inputs || {};
  const pkgBytes = {};
  for (const [inputPath, info] of Object.entries(inputs)) {
    const m =
      inputPath.match(/node_modules[\/\\].pnpm[\/\\](@[^@]+)@/) ||
      inputPath.match(/node_modules[\/\\](@[^\/]+)[\/\\]/);
    const pkg = m ? m[1].replace('+', '/') : 'app';
    pkgBytes[pkg] = (pkgBytes[pkg] || 0) + (info.bytes || 0);
  }
  for (const [pkg, bytes] of Object.entries(pkgBytes).sort((a, b) => b[1] - a[1])) {
    console.log(`     ${bytes.toLocaleString()} B  ${pkg}`);
  }

  if (withinBudget) {
    console.log('The JS payload is within budget.');
    console.log('GATE EXIT=0');
    return 0;
  }

  console.error(
    `error: gzipped payload ${gzipBytes} B exceeds budget ${budgetBytes} B`
  );
  console.error(
    '  Do not raise the budget — add terser-class DCE or remove code instead.'
  );
  console.log('GATE EXIT=1');
  return 1;
}

export { main, loadBudgets, gzipSize, findForbidden, BUDGETS_PATH, BUNDLE_PATH, METAFILE_PATH };

if (import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  main()
    .then((code) => process.exit(code))
    .catch((err) => {
      console.error('error:', err.message);
      process.exit(1);
    });
}
