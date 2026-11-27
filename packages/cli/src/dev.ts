import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'http';
import { watch, type FSWatcher } from 'fs';
import { spawn } from 'child_process';
import { build, type BuildOptions } from 'esbuild';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..', '..', '..');

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
   * is spawned, so the default path is never a simulated compile.
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

/**
 * Incremental AOT + esbuild dev server with a real reload channel.
 *
 * Fast refresh rules:
 *  - Component edits trigger a genuine ngc compile, two esbuild bundles and
 *    an SSE reload event. State is preserved by the shared HmrRuntime.
 *  - Framework/package edits still require a full process restart.
 */
export class DevServer {
  private config: DevConfig;
  private server: Server | null = null;
  private watchers: FSWatcher[] = [];
  private readonly hmrClients = new Set<ServerResponse>();
  private pendingChange: Promise<void> | null = null;
  private hmrVersion = 0;
  private lastCompile: CompileResult | null = null;
  private isRunning = false;

  constructor(config: Partial<DevConfig> = {}) {
    this.config = { ...DEFAULT_DEV_CONFIG, ...config };
  }

  /** Start the dev server: initial compile + bundle, then watch for changes. */
  async start(): Promise<void> {
    if (this.isRunning) return;
    this.isRunning = true;

    await this.compileAndBundle();

    this.server = createServer(async (req, res) => {
      const fs = await import('fs');
      const pathname = (req.url ?? '/').split('?')[0];

      if (pathname === '/') {
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.end(`<script type="module" src="/dev-bundle.js"></script>`);
      } else if (pathname === '/hmr') {
        this.attachHmrClient(req, res);
      } else if (pathname === '/dev-bundle.js') {
        res.writeHead(200, { 'Content-Type': 'application/javascript' });
        res.end(fs.readFileSync(this.config.outfile));
      } else if (pathname === '/dev-bundle.native.js') {
        res.writeHead(200, { 'Content-Type': 'application/javascript' });
        res.end(fs.readFileSync(this.config.nativeOutfile));
      } else {
        res.writeHead(404);
        res.end('not found');
      }
    });

    await new Promise<void>((resolve) => this.server!.listen(this.config.port, resolve));

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

  private publishHmr(): HmrEvent {
    this.hmrVersion += 1;
    const event: HmrEvent = {
      type: 'hmr',
      moduleId: this.config.hmrModuleId,
      version: this.hmrVersion,
      bundles: {
        web: `/dev-bundle.js?v=${this.hmrVersion}`,
        native: `/dev-bundle.native.js?v=${this.hmrVersion}`,
      },
    };
    for (const client of this.hmrClients) {
      client.write(`event: hmr\ndata: ${JSON.stringify(event)}\n\n`);
    }
    return event;
  }

  /** Run one real incremental AOT compile; exposed for tests and evidence. */
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

  /** Handle a file change: real AOT compile, dual rebundle, then reload event. */
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

    const bundleStart = process.hrtime.bigint();
    await this.bundleDev();
    const bundleMs = Number(process.hrtime.bigint() - bundleStart) / 1_000_000;
    const totalMs = Number(process.hrtime.bigint() - start) / 1_000_000;

    this.lastCompile = {
      ok: true,
      componentCompileMs,
      bundleMs,
      totalMs,
      compileCommand: compileResult.compileCommand,
    };

    const event = this.publishHmr();
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

  /** Bundle the compiled output into web ESM and native IIFE dev bundles. */
  private async bundleDev(): Promise<void> {
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
      sourcemap: true,
      minify: false, // dev bundles are NOT minified — speed over size
      treeShaking: true,
      define: { ngDevMode: 'true' },
    };

    await build({ ...base, outfile: this.config.outfile, format: 'esm', platform: 'browser' });
    await build({
      ...base,
      outfile: this.config.nativeOutfile,
      format: 'iife',
      globalName: '__angnativeDevModule',
      platform: 'neutral',
    });
  }

  /** Compile the AOT entry point and bundle it with esbuild. */
  private async compileAndBundle(): Promise<void> {
    const result = await this.compileOnce();
    if (!result.ok) throw new Error(result.error ?? 'initial compile failed');
    await this.bundleDev();
  }

  /** Full restart: used when framework or package files change. */
  async restart(): Promise<void> {
    await this.stop();
    process.exit(42);
  }

  async stop(): Promise<void> {
    this.isRunning = false;
    for (const watcher of this.watchers) watcher.close();
    this.watchers = [];
    if (this.pendingChange) {
      await this.pendingChange.catch(() => undefined);
      this.pendingChange = null;
    }
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
