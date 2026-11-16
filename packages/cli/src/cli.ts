#!/usr/bin/env node
/**
 * cli.ts — AngNative CLI binary entry point.
 * Compiled to dist/cli.js and registered as the `angnative` bin.
 */

import { main } from './commands';

main(process.argv).then((code) => process.exit(code)).catch((err) => {
  console.error('fatal:', err instanceof Error ? err.message : String(err));
  process.exit(1);
});
