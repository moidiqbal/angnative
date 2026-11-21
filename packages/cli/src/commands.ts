#!/usr/bin/env node
/**
 * commands.ts — AngNative CLI dispatcher.
 *
 * Commands:
 *   doctor   Check the toolchain (Node, Angular, TS, Rust, Android SDK, Xcode, NDK)
 *   create   Scaffold a new app (see create.ts)
 *   test     Run the repository test suite
 *   build    Bundle the app for production + size report (AOT compile + esbuild)
 *   dev      Start the dev server with incremental AOT + fast refresh
 *
 * The `build` command is wired here (this track owns commands.ts since no
 * parallel track is modifying it in this session). `doctor`, `dev`, and
 * `build` delegate to their respective modules.
 */

import { doctorMain } from './doctor.js';
import { devMain } from './dev.js';
import { bundleApp } from './bundle.js';
import { createApp } from './create.js';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { pathToFileURL } from 'url';
import { execSync } from 'child_process';

const AOT_TSCONFIG = 'tsconfig.aot.json';
const AOT_OUTPUT_DIR = join(process.cwd(), 'dist', 'aot');
// Real ngc emits the AOT entry at dist/aot/main.js — bundle THAT, not the
// legacy fixture entry (which is used by the separate size-budget gate).
const BUNDLE_ENTRY = join(AOT_OUTPUT_DIR, 'main.js');
const BUNDLE_OUTFILE = join(process.cwd(), 'dist', 'aot', 'bundle.js');

function usage(): void {
  console.log(`
Usage:
  angnative <command> [options]

Commands:
  doctor   Check the toolchain and the pinned package versions
  create <app-name>  Scaffold a new AngNative app
  test     Run the repository test suite
  inspect  Render the native element tree of the AOT fixture as a self-contained page
  build    Bundle the app for production (AOT + esbuild) and report size
  dev      Start the dev server with incremental AOT and fast refresh
`);
}

async function runBuild(): Promise<number> {
  // Step 1: Run Angular AOT compilation
  if (!existsSync(AOT_TSCONFIG)) {
    console.error(`error: AOT tsconfig not found: ${AOT_TSCONFIG}`);
    console.error('  Create tsconfig.aot.json with Angular compiler options.');
    return 1;
  }

  console.log(`[angnative build] running real Angular AOT compile (scripts/compile-aot.mjs)...`);
  try {
    execSync(`node scripts/compile-aot.mjs ${AOT_TSCONFIG}`, {
      stdio: 'inherit',
      cwd: process.cwd(),
    });
  } catch (err) {
    console.error(`error: AOT compilation failed: ${err instanceof Error ? err.message : String(err)}`);
    return 1;
  }

  // Step 2: Bundle the AOT output
  if (!existsSync(BUNDLE_ENTRY)) {
    console.error(`error: AOT fixture not found at ${BUNDLE_ENTRY}`);
    console.error(`  Check that tsconfig.aot.json outputs to ${BUNDLE_ENTRY}`);
    return 1;
  }

  console.log(`[angnative build] bundling ${BUNDLE_ENTRY} -> ${BUNDLE_OUTFILE}`);
  try {
    await bundleApp(BUNDLE_ENTRY, BUNDLE_OUTFILE);
    console.log(`[angnative build] bundle written to ${BUNDLE_OUTFILE}`);
    return 0;
  } catch (err) {
    console.error(`error: build failed: ${err instanceof Error ? err.message : String(err)}`);
    return 1;
  }
}

async function runDev(): Promise<number> {
  await devMain({
    entry: BUNDLE_ENTRY,
    outfile: BUNDLE_OUTFILE.replace('.js', '-dev.js'),
  });
  return 0;
}

function runCreate(argv: string[]): number {
  const [appName] = argv;

  if (!appName) {
    console.error('error: app name is required');
    console.log('  Usage: angnative create <app-name>');
    return 1;
  }

  try {
    const dir = createApp(appName);
    console.log(`[angnative create] scaffolded ${appName} at ${dir}`);
    console.log(`  Next: cd ${appName} && pnpm install`);
    return 0;
  } catch (err) {
    console.error(`error: create failed: ${err instanceof Error ? err.message : String(err)}`);
    return 1;
  }
}

function runInspect(): number {
  console.log('[angnative inspect] open packages/cli/src/inspector.html in a browser.');
  console.log('  The page reads a host-tree JSON snapshot and renders it; no network requests.');
  return 0;
}

function runTest(): number {
  console.log('[angnative test] run `pnpm test` from the repo root.');
  return 0;
}

async function main(argv: string[]): Promise<number> {
  const [, , command, ...rest] = argv;

  switch (command) {
    case 'doctor':
      return doctorMain();
    case 'create':
      return runCreate(rest);
    case 'inspect':
      return runInspect();
    case 'test':
      return runTest();
    case 'build':
      return runBuild();
    case 'dev':
      return runDev();
    case '--help':
    case '-h':
    case undefined:
      usage();
      return command === undefined ? 1 : 0;
    default:
      console.error(`error: unknown command "${command}"`);
      usage();
      return 1;
  }
}

const selfPath = process.argv[1];
if (selfPath && import.meta.url === pathToFileURL(selfPath).href) {
  main(process.argv).then((code) => process.exit(code));
}

export { main, runBuild, runDev, doctorMain, runCreate, runTest };
