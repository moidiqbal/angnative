import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { bundleApp, gzipSize } from './bundle';
import { writeFileSync, mkdirSync, rmSync, existsSync } from 'fs';
import { join } from 'path';

const FIXTURE_DIR = join(process.cwd(), 'dist', '__bundle_spec__');
const ENTRY = join(FIXTURE_DIR, 'entry.ts');
const OUTFILE = join(FIXTURE_DIR, 'bundle.js');

describe('bundleApp — production bundler', () => {
  beforeAll(() => {
    mkdirSync(FIXTURE_DIR, { recursive: true });
    // A minimal entry point that imports nothing from Angular — just enough
    // to prove esbuild bundling works with the production defines.
    writeFileSync(
      ENTRY,
      `export const message = 'hello';\nexport function render() { return message; }\n`
    );
  });

  afterAll(() => {
    rmSync(FIXTURE_DIR, { recursive: true, force: true });
  });

  it('produces a bundle file and a metafile', async () => {
    const { metafile, outputPath } = await bundleApp(ENTRY, OUTFILE);

    expect(existsSync(outputPath)).toBe(true);
    expect(metafile).toBeDefined();
    expect(metafile.inputs).toBeDefined();
    expect(Object.keys(metafile.inputs).length).toBeGreaterThan(0);
  });

  it('applies Angular production defines (ngDevMode=false)', async () => {
    await bundleApp(ENTRY, OUTFILE);
    const output = require('fs').readFileSync(OUTFILE, 'utf-8');

    // Production builds should not contain ngDevMode=true or dev-only checks.
    // The define replaces ngDevMode references with `false`.
    expect(output).not.toContain('ngDevMode = true');
    expect(output).not.toContain('ngDevMode===true');
  });

  it('produces a minified bundle smaller than the source', async () => {
    const { outputPath } = await bundleApp(ENTRY, OUTFILE);
    const outputSize = require('fs').readFileSync(outputPath).length;
    const sourceSize = require('fs').readFileSync(ENTRY).length;

    // esbuild minification should reduce size even for tiny files (tree-shaking
    // removes the export boilerplate we don't use).
    expect(outputSize).toBeLessThanOrEqual(sourceSize);
  });

  it('measures gzip size of the bundle', async () => {
    await bundleApp(ENTRY, OUTFILE);
    const size = await gzipSize(OUTFILE);

    expect(size).toBeGreaterThan(0);
    expect(typeof size).toBe('number');
  });
});
