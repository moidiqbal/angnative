#!/usr/bin/env node
/**
 * commands.ts — AngNative CLI dispatcher.
 *
 * Commands:
 *   doctor   Check the toolchain (Node, Angular, TS, Rust, Android SDK, Xcode, NDK)
 *   create   Scaffold a new app (stub — full scaffolding in a later task)
 *   test     Run the repository test suite
 *   build    Bundle the app for production + size report
 *   dev      Start the dev server with incremental AOT + fast refresh
 *
 * The `build` command is wired here (this track owns commands.ts since no
 * parallel track is modifying it in this session). `doctor`, `dev`, and
 * `build` delegate to their respective modules.
 */

import { doctorMain } from './doctor';
import { devMain } from './dev';
import { bundleApp } from './bundle';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { pathToFileURL } from 'url';

const BUNDLE_ENTRY = join(process.cwd(), 'packages', 'testing', 'fixtures', 'out', 'app.js');
const BUNDLE_OUTFILE = join(
  process.cwd(),
  'packages',
  'testing',
  'fixtures',
  'out',
  'bundle.js'
);

function usage(): void {
  console.log(`
Usage:
  angnative <command> [options]

Commands:
  doctor   Check the toolchain and the pinned package versions
  create   Scaffold a new app
  test     Run the contract suite
  inspect  Render the native element tree of the AOT fixture as a self-contained page
  build    Bundle the app for production and report its size
  dev      Start the dev server with incremental AOT and fast refresh
`);
}

async function runBuild(): Promise<number> {
  if (!existsSync(BUNDLE_ENTRY)) {
    console.error(`error: AOT fixture not found at ${BUNDLE_ENTRY}`);
    console.error('  Run: pnpm run compile:fixtures first');
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

function runCreate(): number {
  console.log('[angnative create] scaffolding is not yet implemented in this track.');
  console.log('  See Task T6 (CLI create command) for full scaffolding.');
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
      return runCreate();
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
