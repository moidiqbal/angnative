#!/usr/bin/env node
/**
 * check-web-size-budget.mjs — CI gate for the AOT browser demo bundle.
 *
 * Reads the AOT bundle + esbuild metafile produced by:
 *   pnpm compile:aot:web && pnpm bundle:aot:web
 *
 * Fails if:
 *   - either artifact is missing
 *   - gzipped app.js exceeds budgets.webDemoGzipBytes
 *   - a forbidden package from budgets.json is in the metafile inputs
 *   - the shipped bundle still contains Angular partial declarations
 *     (`ɵɵngDeclare*`), which means the Angular Linker did not run
 *
 * Exit codes:
 *   0 — within budget and JIT-free
 *   1 — budget exceeded, forbidden package, or unlinked declarations
 */

import { existsSync, readFileSync } from 'fs';
import { createGzip } from 'zlib';
import { join } from 'path';

const REPO_ROOT = process.cwd();

export const BUDGETS_PATH = join(
  REPO_ROOT,
  'packages',
  'cli',
  'src',
  'budgets.json'
);
export const BUNDLE_PATH = join(REPO_ROOT, 'dist', 'web-demo', 'app.js');
export const METAFILE_PATH = join(
  REPO_ROOT,
  'dist',
  'web-demo',
  'app.meta.json'
);

export const PARTIAL_DECLARATION_MARKER = 'ɵɵngDeclare';
export const REGENERATE_HINT =
  'Run: pnpm compile:aot:web && pnpm bundle:aot:web';

/** Load budgets.json — the single source of truth for size limits. */
export function loadBudgets(budgetsPath = BUDGETS_PATH) {
  if (!existsSync(budgetsPath)) {
    throw new Error(`budgets.json not found at ${budgetsPath}`);
  }
  const budgets = JSON.parse(readFileSync(budgetsPath, 'utf-8'));
  if (typeof budgets.webDemoGzipBytes !== 'number') {
    throw new Error('budgets.json is missing numeric webDemoGzipBytes');
  }
  return budgets;
}

/** Gzip a file and return the compressed byte count. */
export function gzipSize(filePath) {
  const input = readFileSync(filePath);
  return new Promise((resolve, reject) => {
    const gzip = createGzip();
    const chunks = [];
    gzip.on('data', (chunk) => chunks.push(chunk));
    gzip.on('end', () => resolve(Buffer.concat(chunks).length));
    gzip.on('error', reject);
    gzip.end(input);
  });
}

/** Scan metafile inputs for forbidden packages. */
export function findForbidden(metafile, forbidden) {
  const inputPaths = Object.keys(metafile?.inputs || {});
  return forbidden.filter((fp) => inputPaths.some((p) => p.includes(fp)));
}

/** Detect Angular partial-compilation declarations in the shipped bundle. */
export function findPartialDeclarations(code) {
  return typeof code === 'string' && code.includes(PARTIAL_DECLARATION_MARKER)
    ? [PARTIAL_DECLARATION_MARKER]
    : [];
}

/**
 * Core gate. Pure with respect to process exit: callers decide the exit code.
 */
export async function runWebSizeGate(options = {}) {
  const budgetsPath = options.budgetsPath ?? BUDGETS_PATH;
  const bundlePath = options.bundlePath ?? BUNDLE_PATH;
  const metafilePath = options.metafilePath ?? METAFILE_PATH;
  const budgets = loadBudgets(budgetsPath);
  const budgetBytes = options.budgetBytes ?? budgets.webDemoGzipBytes;
  const forbiddenPackages = budgets.forbiddenPackages ?? [
    '@angular/compiler',
    'zone.js',
  ];

  const errors = [];
  if (!existsSync(bundlePath)) {
    errors.push({
      code: 'MISSING_ARTIFACT',
      message: `web bundle not found at ${bundlePath}`,
      detail: REGENERATE_HINT,
    });
  }
  if (!existsSync(metafilePath)) {
    errors.push({
      code: 'MISSING_ARTIFACT',
      message: `web bundle metafile missing at ${metafilePath}`,
      detail: REGENERATE_HINT,
    });
  }
  if (errors.length > 0) {
    return {
      ok: false,
      rawBytes: 0,
      gzipBytes: 0,
      budgetBytes,
      forbiddenFound: [],
      partialDeclarationsFound: [],
      errors,
    };
  }

  const bundleBytes = readFileSync(bundlePath);
  const gzipBytes = await gzipSize(bundlePath);
  const metafile = JSON.parse(readFileSync(metafilePath, 'utf-8'));
  const forbiddenFound = findForbidden(metafile, forbiddenPackages);
  const partialDeclarationsFound = findPartialDeclarations(
    bundleBytes.toString('utf8')
  );

  if (gzipBytes > budgetBytes) {
    errors.push({
      code: 'BUDGET_EXCEEDED',
      message: `gzipped web bundle ${gzipBytes} B exceeds budget ${budgetBytes} B by ${gzipBytes - budgetBytes} B`,
      detail: 'Do not raise the budget without evidence — remove code or add DCE.',
    });
  }
  if (forbiddenFound.length > 0) {
    const badInputs = Object.keys(metafile?.inputs || {}).filter((input) =>
      forbiddenFound.some((fp) => input.includes(fp))
    );
    errors.push({
      code: 'FORBIDDEN_PACKAGE',
      message: `web bundle contains forbidden packages: ${forbiddenFound.join(', ')}`,
      detail: badInputs.join('\n'),
    });
  }
  if (partialDeclarationsFound.length > 0) {
    errors.push({
      code: 'UNLINKED_PARTIAL_DECLARATION',
      message: `web bundle contains ${partialDeclarationsFound.join(', ')} markers`,
      detail: 'Angular Linker did not process the partial declarations.',
    });
  }

  return {
    ok: errors.length === 0,
    rawBytes: bundleBytes.length,
    gzipBytes,
    budgetBytes,
    forbiddenFound,
    partialDeclarationsFound,
    errors,
  };
}

/** Human-readable report used by the CLI and evidence docs. */
export function formatReport(result) {
  const lines = [
    `[angnative web] raw bundle: ${result.rawBytes} B`,
    `[angnative web] gzip bundle: ${result.gzipBytes} B (budget ${result.budgetBytes} B / ${(result.budgetBytes / 1024).toFixed(1)} KB, headroom ${result.budgetBytes - result.gzipBytes} B)`,
    `[angnative web] forbidden packages: ${result.forbiddenFound.length}`,
    `[angnative web] unlinked partial declarations: ${result.partialDeclarationsFound.length}`,
  ];
  for (const error of result.errors) {
    lines.push(`error: [${error.code}] ${error.message}`);
    if (error.detail) lines.push(`  ${error.detail}`);
  }
  lines.push(`WEB GATE EXIT=${result.ok ? 0 : 1}`);
  return lines.join('\n');
}

export async function main() {
  const result = await runWebSizeGate();
  console.log(formatReport(result));
  return result.ok ? 0 : 1;
}

if (import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  main()
    .then((code) => process.exit(code))
    .catch((err) => {
      console.error('error:', err.message);
      process.exit(1);
    });
}
