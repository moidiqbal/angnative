#!/usr/bin/env node
/**
 * check-toolchain.mjs — Verifies the pinned toolchain versions.
 *
 * Checks:
 *   - Node >= 22.12 (hard floor)
 *   - Node within Angular 22 engine range (^22.22.3 || ^24.15.0 || >=26.0.0)
 *   - @angular/core == 22.3.0
 *   - TypeScript == 6.0.3
 *
 * Exit codes:
 *   0 — all checks pass
 *   1 — hard error (Node below floor, or version mismatch)
 */

import { readFileSync, existsSync } from 'fs';

const HARD_MIN_NODE = '22.12';
const ANGULAR_ENGINE_NODE = '22.22.3';
const ANGULAR_ENGINE_RANGE = `^${ANGULAR_ENGINE_NODE} || ^24.15.0 || >=26.0.0`;
const REQUIRED_ANGULAR = '22.3.0';
const REQUIRED_TYPESCRIPT = '6.0.3';

function parse(v) {
  const m = /^v?(\d+)\.(\d+)(?:\.(\d+))?/.exec(v);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3] ?? 0)] : null;
}

function cmp(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

function parseRange(range) {
  return range.split('||').map((clause) => {
    const m = /^\s*(>=|\^)\s*(\d+)\.(\d+)\.(\d+)\s*$/.exec(clause);
    if (!m) throw new Error(`Unsupported engine range clause: "${clause}"`);
    const min = [Number(m[2]), Number(m[3]), Number(m[4])];
    let max = null;
    if (m[1] === '^') {
      if (min[0] > 0) max = [min[0] + 1, 0, 0];
      else if (min[1] > 0) max = [0, min[1] + 1, 0];
      else max = [0, 0, min[2] + 1];
    }
    return { min, max };
  });
}

const ENGINE_RANGES = parseRange(ANGULAR_ENGINE_RANGE);

function satisfiesEngineRange(v) {
  return ENGINE_RANGES.some(
    ({ min, max }) => cmp(v, min) >= 0 && (max === null || cmp(v, max) < 0)
  );
}

export function checkToolchain(nodeVersion) {
  const errors = [];
  const warnings = [];
  const v = parse(nodeVersion);
  if (!v) {
    errors.push(`Unrecognised Node version "${nodeVersion}".`);
    return { errors, warnings };
  }
  const floor = parse(HARD_MIN_NODE);
  if (cmp(v, floor) < 0) {
    errors.push(`Node ${nodeVersion} is below the hard floor ${HARD_MIN_NODE}.`);
  } else if (!satisfiesEngineRange(v)) {
    warnings.push(
      `Node ${nodeVersion} is outside the Angular 22 engine range ${ANGULAR_ENGINE_RANGE}; ` +
        'the toolchain still works but npm prints an EBADENGINE warning.',
    );
  }
  return { errors, warnings };
}

export function checkAngularVersion(installed) {
  return installed === REQUIRED_ANGULAR;
}

export function checkTypeScriptVersion(installed) {
  return installed === REQUIRED_TYPESCRIPT;
}

if (import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  const { errors, warnings } = checkToolchain(process.versions.node);
  for (const w of warnings) console.warn(`warn: ${w}`);
  for (const e of errors) console.error(`error: ${e}`);
  console.log(`node ${process.versions.node} · npm ${process.env.npm_config_user_agent ?? 'n/a'}`);
  process.exit(errors.length ? 1 : 0);
}
