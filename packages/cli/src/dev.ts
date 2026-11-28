import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'http';
import { existsSync, readFileSync, renameSync, rmSync, watch, type FSWatcher } from 'fs';
import { spawn, type ChildProcess } from 'child_process';
import { createHash } from 'crypto';
import { build, type BuildOptions } from 'esbuild';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..', '..', '..');
const MAX_BUNDLE_VERSIONS = 16;

/**
 * Configuration for the AngNative dev server.
 */
export interface DevConfig {
  /** Entry point for the AOT-compiled app. */
  entry: string;
  /** Output bundle path for dev (not gzip-checked, not production). */
  outfile: string;
  /** IIFE bundle consumed by the embedded QuickJS runtime. */
  nativeOutfile: string;
  /** Port for the dev server HTTP endpoint (0 = ephemeral, used in tests). */
  port: number;
  /** Maximum time allowed per changed component (ms) — spec: <= 200 ms. */
  maxComponentCompileMs: number;
  /** Maximum inner loop time end to end (ms) — spec: <= 500 ms. */
  maxInnerLoopMs: number;
  /** Real Angular compiler entry point (scripts/compile-aot.mjs). */
  aotScript: string;
  /** tsconfig used for the incremental AOT compile. */
  aotTsconfig: string;
  /** Source files watched for changes. */
  watchPaths: readonly string[];
  /** Module id that HMR replaces in the shared runtime. */
  hmrModuleId: string;
  /**
   * Optional compiler hook for tests. When omitted the real ngc-backed script
   * stays resident in watch mode, so the default path is never a simulated
   * compile. When supplied, the legacy fs.watch + hook path is used instead.
   */
  compileHook?: (tsconfig: string) => Promise<void>;
}

/** Default dev configuration. */
export const DEFAULT_DEV_CONFIG: DevConfig = {
  entry: join(ROOT, 'packages', 'testing', 'fixtures', 'hmr', 'out', 'counter.component.js'),
  outfile: join(ROOT, 'packages', 'testing', 'fixtures', 'hmr', 'out', 'dev-bundle.js'),
  nativeOutfile: join(ROOT, 'packages', 'testing', 'fixtures', 'hmr', 'out', 'dev-bundle.native.js'),
  port: 5173,
  maxComponentCompileMs: 200,
  maxInnerLoopMs: 500,
  aotScript: join(ROOT, 'scripts', 'compile-aot.mjs'),
  aotTsconfig: join(ROOT, 'tsconfig.aot.hmr.json'),
  watchPaths: [join(ROOT, 'packages', 'testing', 'fixtures', 'hmr', 'counter.component.ts')],
  hmrModuleId: 'root',
};

/**
 * Result of a single incremental compile + bundle cycle.
 */
export interface CompileResult {
  ok: boolean;
  componentCompileMs: number;
  bundleMs: number;
  totalMs: number;
  /** Command used for the real ngc invocation (retained as evidence). */
  compileCommand?: string;
  error?: string;
}

/** Reload event published on `/hmr` after a successful cycle. */
export interface HmrEvent {
  type: 'hmr';
  moduleId: string;
  version: number;
  bundles: { web: string; native: string };
}

interface BundlePaths {
  web: string;
  native: string;
}

function stripAnsi(text: string): string {
  return text.replace(/\u001b\[[0-9;]*m/g, '').trim();
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error: unknown) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

/**
 * Incremental AOT + esbuild dev server with a real reload channel.
 *
 * Fast refresh rules:
 *  - The default path keeps the real ngc compiler resident in `--watch` mode.
 *    Only its machine-readable completion line triggers bundle + SSE publish;
 *    raw fs.watch events are deliberately not a trigger.
 *  - Bundles are immutable per version, so a late fetch of vN cannot observe
 *    vN+1 content. State is preserved by the shared HmrRuntime.
 *  - Framework/package edits still require a full process restart.
 */
export class DevServer {
  private config: DevConfig;
  private server: Server | null = null;
  private watchers: FSWatcher[] = [];
  private readonly hmrClients = new Set<ServerResponse>();
  private pendingChange: Promise<void> | null = null;
  private hmrQueue: Promise<void> = Promise.resolve();
  private hmrVersion = 0;
  private lastCompile: CompileResult | null = null;
  private isRunning = false;

  private ngcWatch: ChildProcess | null = null;
  private bundlePaths = new Map<number, BundlePaths>();
  private lastBundledHash: string | null = null;

  constructor(config: Partial<DevConfig> = {}) {
    this.config = { ...DEFAULT_DEV_CONFIG, ...config };
  }

  /** Start the dev server: initial compile + bundle, then watch for changes. */
  async start(): Promise<void> {
    if (this.isRunning) return;
    this.isRunning = true;

    await this.compileAndBundle();

    this.server = createServer((req, res) => {
      const pathname = (req.url ?? '/').split('?')[0];

      if (pathname === '/') {
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.end(`<script type="module" src="/dev-bundle.js"></script>`);
      } else if (pathname === '/hmr') {
        this.attachHmrClient(req, res);
      } else if (pathname === '/dev-bundle.js') {
        this.serveBundle(req.url ?? '', res, 'web');
      } else if (pathname === '/dev-bundle.native.js') {
        this.serveBundle(req.url ?? '', res, 'native');
      } else {
        res.writeHead(404);
        res.end('not found');
      }
    });

    await new Promise<void>((resolve) => this.server!.listen(this.config.port, resolve));

    if (this.config.compileHook) {
      // Deterministic unit-test path: no real compiler child is started.
      for (const path of this.config.watchPaths) {
        this.watchers.push(
          watch(path, () => {
            const change = this.handleChange(path);
            this.pendingChange = change;
            void change.finally(() => {
              if (this.pendingChange === change) this.pendingChange = null;
            });
          }),
        );
      }
    } else {
      // Real path: the resident ngc completion line is the only trigger.
      await this.startNgcWatch();
    }

    console.log(`[angnative dev] server running on http://localhost:${this.getPort()}`);
  }

  /** Port actually bound (non-zero even when the config requests ephemeral). */
  getPort(): number {
    const address = this.server?.address();
    return address && typeof address === 'object' ? address.port : this.config.port;
  }

  /** Current reload version; useful for tests and evidence. */
  getHmrVersion(): number {
    return this.hmrVersion;
  }

  private attachHmrClient(req: IncomingMessage, res: ServerResponse): void {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });
    res.write(`event: ready\ndata: ${JSON.stringify({ version: this.hmrVersion })}\n\n`);
    this.hmrClients.add(res);
    req.on('close', () => this.hmrClients.delete(res));
  }

  private serveBundle(rawUrl: string, res: ServerResponse, kind: keyof BundlePaths): void {
    const url = new URL(rawUrl, 'http://localhost');
    const rawVersion = url.searchParams.get('v');
    const version = rawVersion === null ? this.hmrVersion : Number(rawVersion);
    const paths = this.bundlePaths.get(version);
    const file = paths?.[kind];
    if (!file || !existsSync(file)) {
      res.writeHead(404);
      res.end(`bundle v${version} not found`);
      return;
    }
    res.writeHead(200, {
      'Content-Type': 'application/javascript',
      'Cache-Control': 'no-store',
    });
    res.end(readFileSync(file));
  }

  private publishHmr(version: number): HmrEvent {
    this.hmrVersion = version;
    const event: HmrEvent = {
      type: 'hmr',
      moduleId: this.config.hmrModuleId,
      version,
      bundles: {
        web: `/dev-bundle.js?v=${version}`,
        native: `/dev-bundle.native.js?v=${version}`,
      },
    };
    for (const client of this.hmrClients) {
      client.write(`event: hmr\ndata: ${JSON.stringify(event)}\n\n`);
    }
    return event;
  }

  /** Run one real non-watch incremental AOT compile; exposed for tests/evidence. */
  async compileOnce(): Promise<CompileResult> {
    const start = process.hrtime.bigint();
    try {
      const compileCommand = await this.incrementalCompile();
      return {
        ok: true,
        componentCompileMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        bundleMs: 0,
        totalMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        compileCommand,
      };
    } catch (err) {
      return {
        ok: false,
        componentCompileMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        bundleMs: 0,
        totalMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /** Legacy compileHook path: kept for deterministic DevServer unit tests. */
  private async handleChange(changedPath: string): Promise<void> {
    const start = process.hrtime.bigint();

    const compileResult = await this.compileOnce();
    const componentCompileMs = compileResult.componentCompileMs;
    if (!compileResult.ok) {
      this.lastCompile = {
        ok: false,
        componentCompileMs,
        bundleMs: 0,
        totalMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        compileCommand: compileResult.compileCommand,
        error: compileResult.error,
      };
      console.error(`[angnative dev] compile failed: ${compileResult.error}`);
      return;
    }

    const version = this.hmrVersion + 1;
    const bundleStart = process.hrtime.bigint();
    try {
      await this.bundleDev(version);
    } catch (err) {
      this.lastCompile = {
        ok: false,
        componentCompileMs,
        bundleMs: 0,
        totalMs: Number(process.hrtime.bigint() - start) / 1_000_000,
        compileCommand: compileResult.compileCommand,
        error: err instanceof Error ? err.message : String(err),
      };
      console.error(`[angnative dev] bundle failed: ${this.lastCompile.error}`);
      return;
    }
    const bundleMs = Number(process.hrtime.bigint() - bundleStart) / 1_000_000;
    const totalMs = Number(process.hrtime.bigint() - start) / 1_000_000;
    this.lastBundledHash = this.readEntryHash();

    this.lastCompile = {
      ok: true,
      componentCompileMs,
      bundleMs,
      totalMs,
      compileCommand: compileResult.compileCommand,
    };

    const event = this.publishHmr(version);
    console.log(
      `[angnative dev] ${changedPath}: inner loop ${totalMs.toFixed(0)} ms ` +
        `(compile ${componentCompileMs.toFixed(0)} ms, bundle ${bundleMs.toFixed(0)} ms), ` +
        `reload v${event.version}`,
    );

    if (componentCompileMs > this.config.maxComponentCompileMs) {
      console.warn(
        `[angnative dev] component compile ${componentCompileMs.toFixed(0)} ms exceeds ` +
          `${this.config.maxComponentCompileMs} ms budget`,
      );
    }
    if (totalMs > this.config.maxInnerLoopMs) {
      console.warn(
        `[angnative dev] inner loop ${totalMs.toFixed(0)} ms exceeds ` +
          `${this.config.maxInnerLoopMs} ms budget`,
      );
    }
  }

  /**
   * Real incremental AOT compile. The default path spawns the ngc-backed
   * script; tests may inject a hook without changing the production contract.
   */
  private async incrementalCompile(): Promise<string> {
    const command = `${process.execPath} ${this.config.aotScript} ${this.config.aotTsconfig} --incremental`;
    if (this.config.compileHook) {
      await this.config.compileHook(this.config.aotTsconfig);
      return command;
    }

    await new Promise<void>((resolve, reject) => {
      const child = spawn(process.execPath, [this.config.aotScript, this.config.aotTsconfig, '--incremental'], {
        cwd: process.cwd(),
        stdio: 'ignore',
      });
      child.once('error', reject);
      child.once('close', (code) => {
        if (code === 0) resolve();
        else reject(new Error(`incremental ngc exited with code ${code}`));
      });
    });
    return command;
  }

  private watchCommand(): string {
    return `${process.execPath} ${this.config.aotScript} ${this.config.aotTsconfig} --incremental --watch`;
  }

  private startNgcWatch(): Promise<void> {
    const child = spawn(
      process.execPath,
      [this.config.aotScript, this.config.aotTsconfig, '--incremental', '--watch'],
      { cwd: process.cwd(), stdio: ['ignore', 'pipe', 'pipe'] },
    );
    this.ngcWatch = child;

    let resolveReady!: () => void;
    let rejectReady!: (error: Error) => void;
    let readySettled = false;
    const ready = new Promise<void>((resolve, reject) => {
      resolveReady = resolve;
      rejectReady = reject;
    });

    const consumeLine = (raw: string, isErr: boolean): void => {
      const text = `${raw}\n`;
      if (isErr) process.stderr.write(text);
      else process.stdout.write(text);

      const line = stripAnsi(raw);
      const ok = /^ANB_COMPILE_OK (\d+) (\d+)$/.exec(line);
      if (ok) {
        const seq = Number(ok[1]);
        const ms = Number(ok[2]);
        if (seq === 0) {
          if (!readySettled) {
            readySettled = true;
            resolveReady();
          }
        } else {
          void this.enqueueWatchCycle(seq, ms);
        }
        return;
      }

      const fail = /^ANB_COMPILE_FAIL (\d+) (\d+)$/.exec(line);
      if (fail) {
        const seq = Number(fail[1]);
        if (seq === 0) {
          if (!readySettled) {
            readySettled = true;
            rejectReady(new Error('initial real ngc watch compile failed'));
          }
        } else {
          console.error(`[angnative dev] ngc watch compile ${seq} failed`);
          this.lastCompile = {
            ok: false,
            componentCompileMs: Number(fail[2]),
            bundleMs: 0,
            totalMs: Number(fail[2]),
            compileCommand: this.watchCommand(),
            error: `real ngc watch compile ${seq} failed`,
          };
        }
      }
    };

    const attach = (stream: NodeJS.ReadableStream, isErr: boolean): void => {
      let pending = '';
      stream.on('data', (chunk: Buffer | string) => {
        pending += chunk.toString();
        const lines = pending.split(/\r?\n/);
        pending = lines.pop() ?? '';
        for (const line of lines) consumeLine(line, isErr);
      });
      stream.on('end', () => {
        if (pending) consumeLine(pending, isErr);
        pending = '';
      });
    };
    if (child.stdout) attach(child.stdout, false);
    if (child.stderr) attach(child.stderr, true);

    child.once('error', (err) => {
      if (!readySettled) {
        readySettled = true;
        rejectReady(err);
      }
    });
    child.once('close', (code, signal) => {
      if (!readySettled) {
        readySettled = true;
        rejectReady(new Error(`real ngc watch exited before ready (code ${code ?? signal})`));
      } else if (this.isRunning && code !== 0 && signal === null) {
        console.error(`[angnative dev] real ngc watch exited with code ${code}`);
      }
    });

    return withTimeout(ready, 90_000, 'real ngc watch did not become ready within 90 s').then(() => {
      this.lastBundledHash = this.readEntryHash();
    });
  }

  private readEntryHash(): string | null {
    if (!existsSync(this.config.entry)) return null;
    return createHash('sha256').update(readFileSync(this.config.entry)).digest('hex');
  }

  /** Wait briefly for ngc's emitted JS to settle before bundling it. */
  private async waitForStableEntryHash(): Promise<string | null> {
    let previous = this.readEntryHash();
    if (!previous) return null;
    for (let attempt = 0; attempt < 2; attempt += 1) {
      await delay(10);
      const current = this.readEntryHash();
      if (!current) return null;
      if (current === previous) return current;
      previous = current;
    }
    return previous;
  }

  private enqueueWatchCycle(seq: number, compileMs: number): void {
    this.hmrQueue = this.hmrQueue
      .then(async () => {
        if (!this.isRunning) return;

        const start = process.hrtime.bigint();
        const compileCommand = this.watchCommand();
        const hash = await this.waitForStableEntryHash();
        if (!hash) {
          this.lastCompile = {
            ok: false,
            componentCompileMs: compileMs,
            bundleMs: 0,
            totalMs: Number(process.hrtime.bigint() - start) / 1_000_000,
            compileCommand,
            error: 'AOT entry output missing after ngc watch completion',
          };
          console.error(this.lastCompile.error);
          return;
        }
        if (hash === this.lastBundledHash) {
          console.log(`[angnative dev] ngc watch v${seq} produced no output change; HMR skipped`);
          return;
        }

        const version = this.hmrVersion + 1;
        const bundleStart = process.hrtime.bigint();
        await this.bundleDev(version);
        const bundleMs = Number(process.hrtime.bigint() - bundleStart) / 1_000_000;
        const totalMs = Number(process.hrtime.bigint() - start) / 1_000_000;
        this.lastBundledHash = hash;

        this.lastCompile = {
          ok: true,
          componentCompileMs: compileMs,
          bundleMs,
          totalMs,
          compileCommand,
        };
        if (!this.isRunning) return; // stop() raced the bundle; never publish.
        const event = this.publishHmr(version);
        console.log(
          `[angnative dev] ngc watch v${seq}: loop ${totalMs.toFixed(0)} ms ` +
            `(compile ${compileMs.toFixed(0)} ms, bundle ${bundleMs.toFixed(0)} ms), ` +
            `reload v${event.version}`,
        );

        if (compileMs > this.config.maxComponentCompileMs) {
          console.warn(
            `[angnative dev] component compile ${compileMs.toFixed(0)} ms exceeds ` +
              `${this.config.maxComponentCompileMs} ms budget`,
          );
        }
        if (totalMs > this.config.maxInnerLoopMs) {
          console.warn(
            `[angnative dev] inner loop ${totalMs.toFixed(0)} ms exceeds ` +
              `${this.config.maxInnerLoopMs} ms budget`,
          );
        }
      })
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : String(err);
        this.lastCompile = {
          ok: false,
          componentCompileMs: compileMs,
          bundleMs: 0,
          totalMs: compileMs,
          compileCommand: this.watchCommand(),
          error: message,
        };
        console.error(`[angnative dev] ngc watch cycle ${seq} failed: ${message}`);
      });
  }

  private versionedPath(base: string, version: number): string {
    return /\.js$/.test(base) ? base.replace(/\.js$/, `.v${version}.js`) : `${base}.v${version}`;
  }

  /** Bundle the compiled output into immutable per-version web/native bundles. */
  private async bundleDev(version: number): Promise<BundlePaths> {
    const webFinal = this.versionedPath(this.config.outfile, version);
    const nativeFinal = this.versionedPath(this.config.nativeOutfile, version);
    const webTemp = `${webFinal}.tmp`;
    const nativeTemp = `${nativeFinal}.tmp`;
    rmSync(webTemp, { force: true });
    rmSync(nativeTemp, { force: true });

    const hmrEntry = `
import * as app from ${JSON.stringify(this.config.entry)};
const factory = typeof app['bootstrap'] === 'function' ? () => app['bootstrap']() : () => app;
const hooks = {
  snapshot: (instance) => instance && typeof instance.snapshot === 'function'
    ? instance.snapshot() : undefined,
  restore: (instance, state) => {
    if (instance && typeof instance.restore === 'function') instance.restore(state);
  },
  dispose: (instance) => {
    if (instance && typeof instance.dispose === 'function') instance.dispose();
  }
};
export const hmrModule = { factory, hooks };
`;

    const base: Omit<BuildOptions, 'format' | 'outfile'> = {
      stdin: {
        contents: hmrEntry,
        resolveDir: dirname(this.config.entry),
        sourcefile: 'hmr-entry.js',
        loader: 'js',
      },
      bundle: true,
      target: 'es2022',
      // HMR payloads are fetched on every edit; source maps are omitted so the
      // hot path stays small. Non-HMR builds keep their normal source maps.
      sourcemap: false,
      minify: false, // dev bundles are NOT minified — speed over size
      treeShaking: true,
      define: { ngDevMode: 'true' },
    };

    try {
      await Promise.all([
        build({ ...base, outfile: webTemp, format: 'esm', platform: 'browser' }),
        build({
          ...base,
          outfile: nativeTemp,
          format: 'iife',
          globalName: '__angnativeDevModule',
          platform: 'neutral',
        }),
      ]);
      renameSync(webTemp, webFinal);
      renameSync(nativeTemp, nativeFinal);
    } catch (err) {
      rmSync(webTemp, { force: true });
      rmSync(nativeTemp, { force: true });
      throw err;
    }

    const paths = { web: webFinal, native: nativeFinal };
    this.bundlePaths.set(version, paths);
    this.pruneBundlePaths();
    return paths;
  }

  private pruneBundlePaths(): void {
    const minimum = this.hmrVersion - MAX_BUNDLE_VERSIONS;
    for (const [version, paths] of this.bundlePaths) {
      if (version >= minimum) continue;
      this.bundlePaths.delete(version);
      rmSync(paths.web, { force: true });
      rmSync(paths.native, { force: true });
    }
  }

  /** Compile the AOT entry point and bundle it with esbuild. */
  private async compileAndBundle(): Promise<void> {
    const result = await this.compileOnce();
    if (!result.ok) throw new Error(result.error ?? 'initial compile failed');
    await this.bundleDev(0);
    this.lastBundledHash = this.readEntryHash();
  }

  /** Full restart: used when framework or package files change. */
  async restart(): Promise<void> {
    await this.stop();
    process.exit(42);
  }

  private async stopNgcWatch(): Promise<void> {
    const child = this.ngcWatch;
    if (!child) return;
    this.ngcWatch = null;

    if (process.platform === 'win32' && child.pid) {
      // child.kill() does not kill descendants reliably on Windows. ngc may
      // have spawned its own workers, so terminate the whole tree first.
      await new Promise<void>((resolve) => {
        const killer = spawn('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
        killer.once('error', resolve);
        killer.once('close', resolve);
      });
    } else {
      child.kill('SIGTERM');
    }

    if (child.exitCode === null && child.signalCode === null) {
      await Promise.race([
        new Promise<void>((resolve) => child.once('close', resolve)),
        delay(5_000),
      ]);
    }
  }

  async stop(): Promise<void> {
    this.isRunning = false;
    for (const watcher of this.watchers) watcher.close();
    this.watchers = [];
    if (this.pendingChange) {
      await this.pendingChange.catch(() => undefined);
      this.pendingChange = null;
    }
    await this.stopNgcWatch();
    await this.hmrQueue.catch(() => undefined);
    for (const client of this.hmrClients) client.end();
    this.hmrClients.clear();
    if (this.server) {
      await new Promise<void>((resolve) => this.server!.close(() => resolve()));
      this.server = null;
    }
  }

  getLastCompile(): CompileResult | null {
    return this.lastCompile;
  }
}

/**
 * CLI entry point: `angnative dev` (or `nativeangular dev`).
 */
export async function devMain(configOverride: Partial<DevConfig> = {}): Promise<void> {
  const server = new DevServer(configOverride);

  process.on('SIGINT', async () => {
    await server.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await server.stop();
    process.exit(0);
  });

  await server.start();
}
