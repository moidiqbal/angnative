import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { DevServer, DEFAULT_DEV_CONFIG, type DevConfig } from './dev';
import { writeFileSync, mkdirSync, rmSync, existsSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// Resolve the shared HMR fixture from this file, not from the test runner's
// cwd: `pnpm test` starts at the workspace root, while package-scoped runs
// start in packages/cli.
const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const HMR_FIXTURE_OUT = join(
  REPO_ROOT,
  'packages',
  'testing',
  'fixtures',
  'hmr',
  'out',
  'counter.component.js',
);
const FIXTURE_DIR = join(process.cwd(), 'dist', '__dev_spec__');
const ENTRY = join(FIXTURE_DIR, 'entry.js');
const WATCHED = join(FIXTURE_DIR, 'watched.ts');

describe('DevServer — incremental AOT + fast refresh', () => {
  beforeAll(() => {
    mkdirSync(FIXTURE_DIR, { recursive: true });
    writeFileSync(ENTRY, 'export const x = 1;');
    writeFileSync(WATCHED, 'export const x = 1;');
  });

  afterAll(() => {
    rmSync(FIXTURE_DIR, { recursive: true, force: true });
  });

  it('has sensible default budgets (<= 200 ms component, <= 500 ms inner loop)', () => {
    expect(DEFAULT_DEV_CONFIG.maxComponentCompileMs).toBe(200);
    expect(DEFAULT_DEV_CONFIG.maxInnerLoopMs).toBe(500);
  });

  it('constructs with custom config', () => {
    const config: Partial<DevConfig> = {
      entry: ENTRY,
      outfile: join(FIXTURE_DIR, 'bundle.js'),
      port: 9876,
      maxComponentCompileMs: 150,
      maxInnerLoopMs: 400,
    };
    const server = new DevServer(config);
    expect(server.getLastCompile()).toBeNull();
  });

  it('reports last compile as null before any compile', () => {
    const server = new DevServer({ entry: ENTRY });
    expect(server.getLastCompile()).toBeNull();
  });

  it('stop on an unstarted server does not throw', async () => {
    const server = new DevServer({ entry: ENTRY });
    await expect(server.stop()).resolves.toBeUndefined();
  });

  it('invokes the real ngc-backed script for an incremental compile', async () => {
    const server = new DevServer({
      entry: ENTRY,
      outfile: join(FIXTURE_DIR, 'real-ngc-bundle.js'),
      nativeOutfile: join(FIXTURE_DIR, 'real-ngc-bundle.native.js'),
    });
    const result = await server.compileOnce();
    expect(result.ok, result.error).toBe(true);
    expect(result.compileCommand).toContain('compile-aot.mjs');
    expect(result.compileCommand).toContain('--incremental');

    const compiled = readFileSync(HMR_FIXTURE_OUT, 'utf-8');
    expect(compiled).toMatch(/ɵɵdefineComponent|defineComponent/);
  }, 60_000);

  it('watches source files, rebundles and publishes a versioned HMR event', async () => {
    const server = new DevServer({
      entry: ENTRY,
      outfile: join(FIXTURE_DIR, 'hmr-bundle.js'),
      nativeOutfile: join(FIXTURE_DIR, 'hmr-bundle.native.js'),
      port: 0,
      watchPaths: [WATCHED],
      hmrModuleId: 'counter',
      compileHook: async () => {
        writeFileSync(ENTRY, `export const x = ${Date.now()};`);
      },
    });
    await server.start();

    const controller = new AbortController();
    try {
      const response = await fetch(`http://127.0.0.1:${server.getPort()}/hmr`, {
        signal: controller.signal,
      });
      expect(response.headers.get('content-type')).toContain('text/event-stream');
      const reader = response.body!.getReader();
      const decoder = new TextDecoder();

      const hmrEvent = new Promise<string>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('HMR event timed out')), 10_000);
        (async () => {
          let buffer = '';
          while (!buffer.includes('event: hmr')) {
            const { value, done } = await reader.read();
            if (done) {
              clearTimeout(timer);
              reject(new Error('HMR stream closed before event'));
              return;
            }
            buffer += decoder.decode(value, { stream: true });
          }
          clearTimeout(timer);
          resolve(buffer);
        })().catch(reject);
      });

      writeFileSync(WATCHED, 'export const x = 2;');
      const payload = await hmrEvent;
      const dataLine = payload
        .split('\n')
        .find((line) => line.startsWith('data: ') && line.includes('"type":"hmr"'));
      expect(dataLine).toBeDefined();

      const event = JSON.parse(dataLine!.slice('data: '.length).trim());
      expect(event.type).toBe('hmr');
      expect(event.moduleId).toBe('counter');
      expect(event.version).toBeGreaterThanOrEqual(1);
      expect(event.bundles.web).toMatch(/^\/dev-bundle\.js\?v=\d+$/);
      expect(event.bundles.native).toMatch(/^\/dev-bundle\.native\.js\?v=\d+$/);

      const last = server.getLastCompile();
      expect(last?.ok).toBe(true);
      expect(last?.bundleMs).toBeGreaterThan(0);
      expect(existsSync(join(FIXTURE_DIR, 'hmr-bundle.native.js'))).toBe(true);
    } finally {
      controller.abort();
      await server.stop();
    }
  }, 30_000);
});
