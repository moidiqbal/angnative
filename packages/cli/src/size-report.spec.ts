import { describe, it, expect } from 'vitest';
import { buildSizeReport, type SizeReport, type PackageSize } from './size-report';

function makeMetafile(inputs: Record<string, { bytes: number; inputs?: Record<string, unknown> }>): Parameters<typeof buildSizeReport>[0] {
  return { inputs } as unknown as Parameters<typeof buildSizeReport>[0];
}

describe('buildSizeReport — per-package size breakdown', () => {
  it('aggregates bytes by package from metafile inputs', () => {
    const metafile = makeMetafile({
      'node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs': {
        bytes: 41046,
        inputs: {},
      },
      'node_modules/.pnpm/rxjs@7.8.2/node_modules/rxjs/dist/esm/index.mjs': {
        bytes: 10995,
        inputs: {},
      },
      '/abs/path/to/packages/testing/fixtures/out/app.js': {
        bytes: 636,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(metafile, 14831, 61440);

    expect(report.totalBytes).toBe(41046 + 10995 + 636);
    expect(report.totalGzipBytes).toBe(14831);

    const core = report.packages.find((p) => p.package === '@angular/core');
    expect(core).toBeDefined();
    expect(core!.bytes).toBe(41046);

    const app = report.packages.find((p) => p.package === 'app');
    expect(app).toBeDefined();
    expect(app!.bytes).toBe(636);
  });

  it('sorts packages by bytes descending', () => {
    const metafile = makeMetafile({
      'node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs': {
        bytes: 100,
        inputs: {},
      },
      'node_modules/.pnpm/rxjs@7.8.2/node_modules/rxjs/dist/esm/index.mjs': {
        bytes: 1000,
        inputs: {},
      },
      'node_modules/.pnpm/@angnative+cli@0.1.0/node_modules/@angnative/cli/dist/index.mjs': {
        bytes: 10,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(metafile, 500, 61440);

    expect(report.packages[0]!.package).toBe('rxjs');
    expect(report.packages[1]!.package).toBe('@angular/core');
    expect(report.packages[2]!.package).toBe('@angnative/cli');
  });

  it('detects forbidden packages (@angular/compiler)', () => {
    const metafile = makeMetafile({
      'node_modules/.pnpm/@angular+compiler@22.2.5/node_modules/@angular/compiler/fesm2022/compiler.mjs': {
        bytes: 1222744,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(
      metafile,
      227530,
      61440,
      ['@angular/compiler', 'zone.js']
    );

    expect(report.forbiddenFound).toContain('@angular/compiler');
    expect(report.withinBudget).toBe(false);
  });

  it('detects forbidden packages (zone.js)', () => {
    const metafile = makeMetafile({
      'node_modules/zone.js/dist/zone.mjs': {
        bytes: 50000,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(
      metafile,
      15000,
      61440,
      ['@angular/compiler', 'zone.js']
    );

    expect(report.forbiddenFound).toContain('zone.js');
    expect(report.withinBudget).toBe(false);
  });

  it('passes when within budget and no forbidden packages', () => {
    const metafile = makeMetafile({
      'node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs': {
        bytes: 41046,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(metafile, 14831, 61440, [
      '@angular/compiler',
      'zone.js',
    ]);

    expect(report.withinBudget).toBe(true);
    expect(report.forbiddenFound).toEqual([]);
    expect(report.budgetBytes).toBe(61440);
  });

  it('fails when gzip exceeds budget', () => {
    const metafile = makeMetafile({
      'node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs': {
        bytes: 41046,
        inputs: {},
      },
    });

    const report: SizeReport = buildSizeReport(metafile, 70000, 61440, [
      '@angular/compiler',
      'zone.js',
    ]);

    expect(report.withinBudget).toBe(false);
  });
});
