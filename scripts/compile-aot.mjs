/**
 * compile-aot.mjs — REAL Angular AOT (ngc) compilation.
 *
 * Why this exists: `npx ngc` in this workspace resolves to a shadowing 0.1.0
 * binary that only emits plain JS (no Ivy component definitions). The genuine
 * Angular compiler-cli lives inside pnpm's virtual store. We resolve it from
 * `@angular/compiler-cli/package.json`'s `bin.ngc` field instead, so the output
 * under dist/aot contains ɵɵdefineComponent / template functions.
 *
 * Usage:
 *   node scripts/compile-aot.mjs [tsconfig.aot.json] [--incremental] [--watch]
 *
 * `--watch` keeps the real ngc process resident and emits machine-readable
 * completion lines on stdout:
 *   ANB_COMPILE_OK <seq> <ms>
 *   ANB_COMPILE_FAIL <seq> <ms>
 * Sequence 0 is the initial startup compile; later sequences are incremental
 * recompiles. DevServer treats only these lines as HMR triggers.
 */

import { execFileSync, spawn } from 'child_process';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join, resolve } from 'path';

const require = createRequire(import.meta.url);
const args = process.argv.slice(2);
const AOT_TSCONFIG = args[0] && !args[0].startsWith('--') ? args.shift() : 'tsconfig.aot.json';
const EXTRA_NGC_ARGS = args;
const WATCH = EXTRA_NGC_ARGS.includes('--watch');

if (!existsSync(AOT_TSCONFIG)) {
  console.error(`error: AOT tsconfig not found: ${AOT_TSCONFIG}`);
  process.exit(1);
}

function resolveAngularNgc() {
  const pkgJson = require.resolve('@angular/compiler-cli/package.json');
  const pkg = JSON.parse(readFileSync(pkgJson, 'utf-8'));
  const bin = pkg.bin?.ngc || './bundles/src/bin/ngc.js';
  return resolve(dirname(pkgJson), bin);
}

const ngc = resolveAngularNgc();
if (!existsSync(ngc)) {
  console.error(`error: Angular ngc not found at: ${ngc}`);
  process.exit(1);
}

const tsconfig = JSON.parse(readFileSync(AOT_TSCONFIG, 'utf-8'));
// TypeScript resolves outDir relative to the tsconfig file, not the shell cwd.
const outDir = resolve(dirname(resolve(AOT_TSCONFIG)), tsconfig.compilerOptions?.outDir ?? 'dist/aot');

function hasIvyDefinition(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (hasIvyDefinition(path)) return true;
    } else if (entry.name.endsWith('.js')) {
      if (/ɵɵdefineComponent|defineComponent/.test(readFileSync(path, 'utf-8'))) {
        return true;
      }
    }
  }
  return false;
}

function verifyIvyDefinition() {
  if (!existsSync(outDir)) {
    console.error(`error: AOT output directory missing: ${outDir}`);
    return false;
  }
  if (!hasIvyDefinition(outDir)) {
    console.error('error: AOT output has no Ivy component definitions — compiler was not real ngc');
    return false;
  }
  return true;
}

function runOnce() {
  console.log(`[angnative aot] Angular compiler-cli: ${ngc}`);
  console.log(`[angnative aot] compiling with ${AOT_TSCONFIG}...`);
  try {
    execFileSync(process.execPath, [ngc, '--project', AOT_TSCONFIG, ...EXTRA_NGC_ARGS], {
      stdio: 'inherit',
      cwd: process.cwd(),
    });
  } catch (err) {
    console.error(`error: AOT compilation failed: ${err.message}`);
    process.exit(1);
  }

  if (!verifyIvyDefinition()) process.exit(1);
  console.log('[angnative aot] AOT compilation complete (Ivy definitions verified).');
}

function stripAnsi(text) {
  // ngc watch status lines are colourised; strip codes only for parsing.
  return text.replace(/\u001b\[[0-9;]*m/g, '').trim();
}

function attachLineParser(stream, forward) {
  let pending = '';
  stream.on('data', (chunk) => {
    pending += chunk.toString();
    const lines = pending.split(/\r?\n/);
    pending = lines.pop() ?? '';
    for (const line of lines) handleWatchLine(line, forward);
  });
  stream.on('end', () => {
    if (pending) handleWatchLine(pending, forward);
    pending = '';
  });
}

let watchSeq = 0;
let cycleStart = process.hrtime.bigint();
let hasErrors = false;
let initialVerified = false;

function emitWatchResult(ok, ms) {
  const kind = ok ? 'OK' : 'FAIL';
  process.stdout.write(`ANB_COMPILE_${kind} ${watchSeq} ${Math.round(ms)}\n`);
  watchSeq += 1;
  hasErrors = false;
}

function handleWatchLine(raw, forward) {
  forward(`${raw}\n`);
  const line = stripAnsi(raw);

  if (/File change detected\.\s*Starting incremental compilation\./.test(line)) {
    cycleStart = process.hrtime.bigint();
    hasErrors = false;
    return;
  }

  if (/\berror (?:TS|NG)\d+:/.test(line) || /\bFound \d+ errors?\b/.test(line)) {
    hasErrors = true;
  }

  const finished =
    /Compilation complete\.\s*Watching for file changes\.?/.test(line) ||
    /\bFound \d+ errors?\.\s*Watching for file changes\.?/.test(line);
  if (!finished) return;

  const ms = Number(process.hrtime.bigint() - cycleStart) / 1_000_000;
  if (!initialVerified) {
    // One-time provenance gate: the first real ngc output must contain Ivy.
    if (!verifyIvyDefinition()) {
      emitWatchResult(false, ms);
      process.exitCode = 1;
      return;
    }
    initialVerified = true;
    console.log('[angnative aot] AOT watch compile complete (Ivy definitions verified).');
  }
  emitWatchResult(!hasErrors, ms);
}

function runWatch() {
  console.log(`[angnative aot] Angular compiler-cli: ${ngc}`);
  console.log(`[angnative aot] watching with ${AOT_TSCONFIG} (real ngc --watch)...`);

  const child = spawn(process.execPath, [ngc, '--project', AOT_TSCONFIG, ...EXTRA_NGC_ARGS], {
    cwd: process.cwd(),
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  let shuttingDown = false;
  attachLineParser(child.stdout, (text) => process.stdout.write(text));
  attachLineParser(child.stderr, (text) => process.stderr.write(text));

  child.once('error', (err) => {
    console.error(`error: ngc watch failed to start: ${err.message}`);
    process.exit(1);
  });
  child.once('close', (code, signal) => {
    if (!shuttingDown) {
      process.stdout.write(`ANB_COMPILE_FAIL ${watchSeq} 0\n`);
      process.exitCode = code ?? (signal ? 1 : 0);
    }
  });

  const shutdown = () => {
    shuttingDown = true;
    child.kill();
  };
  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

if (WATCH) runWatch();
else runOnce();
