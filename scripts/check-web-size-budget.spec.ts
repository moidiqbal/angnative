import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'fs';
import { join } from 'path';
import {
  BUDGETS_PATH,
  REGENERATE_HINT,
  findForbidden,
  findPartialDeclarations,
  gzipSize,
  loadBudgets,
  runWebSizeGate,
} from '../scripts/check-web-size-budget.mjs';

const SPEC_DIR = join(process.cwd(), 'dist', '__web_size_budget_spec__');
const BUNDLE = join(SPEC_DIR, 'app.js');
const METAFILE = join(SPEC_DIR, 'app.meta.json');
const MISSING = join(SPEC_DIR, 'missing.js');

function metafile(inputs: Record<string, { bytes: number }>) {
  return { inputs, outputs: { 'app.js': { bytes: 100 } } };
}

function writeFixtures(bundleText: string, inputs: Record<string, { bytes: number }>) {
  mkdirSync(SPEC_DIR, { recursive: true });
  writeFileSync(BUNDLE, bundleText, 'utf-8');
  writeFileSync(METAFILE, JSON.stringify(metafile(inputs)), 'utf-8');
}

const allowedInputs = {
  'node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs': {
    bytes: 41046,
  },
};

describe('web size budget — configuration', () => {
  it('loads the dedicated 128 KB gzip budget and forbidden packages', () => {
    const budgets = loadBudgets(BUDGETS_PATH);
    expect(budgets.webDemoGzipBytes).toBe(128000);
    expect(budgets.forbiddenPackages).toContain('@angular/compiler');
    expect(budgets.forbiddenPackages).toContain('zone.js');
  });
});

describe('web size budget — helpers', () => {
  beforeAll(() => {
    writeFixtures('hello world'.repeat(100), allowedInputs);
  });

  afterAll(() => {
    rmSync(SPEC_DIR, { recursive: true, force: true });
  });

  it('gzipSize returns a positive count smaller than repetitive raw text', async () => {
    const raw = readFileSync(BUNDLE).length;
    const gz = await gzipSize(BUNDLE);
    expect(gz).toBeGreaterThan(0);
    expect(gz).toBeLessThan(raw);
  });

  it('findForbidden detects banned metafile inputs', () => {
    const found = findForbidden(
      metafile({
        'node_modules/.pnpm/@angular+compiler@22.2.5/node_modules/@angular/compiler/fesm2022/compiler.mjs': {
          bytes: 1,
        },
      }),
      ['@angular/compiler', 'zone.js']
    );
    expect(found).toContain('@angular/compiler');
  });

  it('findPartialDeclarations detects unlinked Angular output', () => {
    expect(findPartialDeclarations('ɵɵngDeclareComponent({})')).toEqual([
      'ɵɵngDeclare',
    ]);
    expect(findPartialDeclarations('ɵɵdefineComponent({})')).toEqual([]);
  });
});

describe('web size budget — end-to-end gate', () => {
  beforeAll(() => {
    writeFixtures('export const x = 1;', allowedInputs);
  });

  afterAll(() => {
    rmSync(SPEC_DIR, { recursive: true, force: true });
  });

  it('passes for a JIT-free bundle inside the budget', async () => {
    const result = await runWebSizeGate({
      budgetsPath: BUDGETS_PATH,
      bundlePath: BUNDLE,
      metafilePath: METAFILE,
    });
    expect(result.ok).toBe(true);
    expect(result.errors).toEqual([]);
    expect(result.gzipBytes).toBeLessThanOrEqual(result.budgetBytes);
  });

  it('fails with BUDGET_EXCEEDED when gzip is above budget', async () => {
    const result = await runWebSizeGate({
      budgetsPath: BUDGETS_PATH,
      bundlePath: BUNDLE,
      metafilePath: METAFILE,
      budgetBytes: 1,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.map((e) => e.code)).toContain('BUDGET_EXCEEDED');
  });

  it('fails with FORBIDDEN_PACKAGE for @angular/compiler input', async () => {
    writeFixtures('export const x = 1;', {
      ...allowedInputs,
      'node_modules/.pnpm/@angular+compiler@22.2.5/node_modules/@angular/compiler/fesm2022/compiler.mjs': {
        bytes: 1000,
      },
    });
    const result = await runWebSizeGate({
      budgetsPath: BUDGETS_PATH,
      bundlePath: BUNDLE,
      metafilePath: METAFILE,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.map((e) => e.code)).toContain('FORBIDDEN_PACKAGE');
    expect(result.forbiddenFound).toContain('@angular/compiler');
  });

  it('fails with UNLINKED_PARTIAL_DECLARATION for ɵɵngDeclare output', async () => {
    writeFixtures('ɵɵngDeclareFactory({});', allowedInputs);
    const result = await runWebSizeGate({
      budgetsPath: BUDGETS_PATH,
      bundlePath: BUNDLE,
      metafilePath: METAFILE,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.map((e) => e.code)).toContain(
      'UNLINKED_PARTIAL_DECLARATION'
    );
  });

  it('fails with MISSING_ARTIFACT and a regeneration hint', async () => {
    const result = await runWebSizeGate({
      budgetsPath: BUDGETS_PATH,
      bundlePath: MISSING,
      metafilePath: METAFILE,
    });
    expect(result.ok).toBe(false);
    const missing = result.errors.find((e) => e.code === 'MISSING_ARTIFACT');
    expect(missing?.detail).toBe(REGENERATE_HINT);
  });
});
