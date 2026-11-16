import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { DevServer, DEFAULT_DEV_CONFIG, type DevConfig } from './dev';
import { writeFileSync, mkdirSync, rmSync, existsSync } from 'fs';
import { join } from 'path';

const FIXTURE_DIR = join(process.cwd(), 'dist', '__dev_spec__');
const ENTRY = join(FIXTURE_DIR, 'entry.js');

describe('DevServer — incremental AOT + fast refresh', () => {
  beforeAll(() => {
    mkdirSync(FIXTURE_DIR, { recursive: true });
    writeFileSync(ENTRY, 'export const x = 1;');
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
});
