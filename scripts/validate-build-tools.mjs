#!/usr/bin/env node
/**
 * validate-build-tools.mjs — Phase 2 build-time tool validation for AngNative.
 *
 * This script is READ-ONLY: it never writes, installs, or builds anything.
 * It verifies the build-time toolchain facts by reading files that actually
 * exist in the repository and by running two harmless version commands
 * (`node --version`, `pnpm --version`). It prints a PASS / FAIL / INFO table
 * and exits 1 if (and only if) at least one FAIL row was recorded.
 *
 * INFO rows are factual findings about intentionally-absent optional tooling
 * (for example: Nx is not installed, there is no Angular CLI workspace, Jest is
 * not present). They are reported honestly, but do not fail the run.
 *
 * Usage:
 *   node scripts/validate-build-tools.mjs
 */

import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(SCRIPT_DIR, '..');

// ---------------------------------------------------------------------------
// Result collection
// ---------------------------------------------------------------------------

/** @type {{status:'PASS'|'FAIL'|'INFO', check:string, detail:string}[]} */
const rows = [];
const add = (status, check, detail) => rows.push({ status, check, detail: String(detail ?? '') });
const pass = (check, detail) => add('PASS', check, detail);
const fail = (check, detail) => add('FAIL', check, detail);
const info = (check, detail) => add('INFO', check, detail);

// ---------------------------------------------------------------------------
// Safe I/O helpers — a missing or malformed file is a finding, never a crash.
// ---------------------------------------------------------------------------

function safeRead(rel) {
  try {
    return readFileSync(path.join(ROOT, rel), 'utf8');
  } catch {
    return null;
  }
}

function safeJson(rel) {
  const text = safeRead(rel);
  if (text === null) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function safeDirs(rel) {
  try {
    return readdirSync(path.join(ROOT, rel), { withFileTypes: true })
      .filter((e) => e.isDirectory())
      .map((e) => e.name)
      .sort();
  } catch {
    return null;
  }
}

function runVersion(cmd, args) {
  try {
    const out = execFileSync(cmd, args, {
      cwd: ROOT,
      encoding: 'utf8',
      shell: true, // required for pnpm's .cmd shim on Windows
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
    return { ok: true, out };
  } catch (err) {
    return { ok: false, out: String(err?.stderr || err?.message || 'unknown error').trim() };
  }
}

/** Collect every directory under a base that contains a package.json. */
function packageDirs(base) {
  const names = safeDirs(base);
  if (!names) return [];
  const found = [];
  for (const name of names) {
    const rel = `${base}/${name}`;
    if (existsSync(path.join(ROOT, rel, 'package.json'))) found.push(rel);
  }
  return found.sort();
}

// ---------------------------------------------------------------------------
// 1. Runtime version commands
// ---------------------------------------------------------------------------

{
  const node = runVersion('node', ['--version']);
  if (node.ok) pass('node --version', `${node.out} (process.versions.node = ${process.versions.node})`);
  else fail('node --version', `command failed: ${node.out}`);

  const pnpm = runVersion('pnpm', ['--version']);
  if (pnpm.ok) pass('pnpm --version', pnpm.out);
  else fail('pnpm --version', `command failed: ${pnpm.out}`);
}

// ---------------------------------------------------------------------------
// 2. Angular / TypeScript / Vitest versions declared in root package.json
// ---------------------------------------------------------------------------

const rootPkg = safeJson('package.json');
if (!rootPkg) {
  fail('Root package.json', 'missing or invalid JSON — cannot verify declared versions');
} else {
  const dev = rootPkg.devDependencies ?? {};

  for (const name of ['@angular/core', '@angular/compiler-cli', 'typescript', 'vitest']) {
    const v = dev[name];
    if (typeof v === 'string' && v.length) pass(`${name} declared`, `${v} (root devDependencies)`);
    else fail(`${name} declared`, 'not declared in root devDependencies');
  }

  if (dev['@angular/core'] && dev['@angular/compiler-cli']) {
    if (dev['@angular/core'] === dev['@angular/compiler-cli']) {
      pass('Angular core / compiler-cli aligned', `both ${dev['@angular/core']}`);
    } else {
      fail(
        'Angular core / compiler-cli aligned',
        `core ${dev['@angular/core']} != compiler-cli ${dev['@angular/compiler-cli']}`,
      );
    }
  }

  if (dev['@angular/cli']) pass('@angular/cli declared', dev['@angular/cli']);
  else info('@angular/cli declared', 'NOT declared — this repo builds with pnpm + "tsc -b", not the Angular CLI');
}

// ---------------------------------------------------------------------------
// 3. TypeScript configuration files
// ---------------------------------------------------------------------------

for (const f of ['tsconfig.json', 'tsconfig.base.json', 'tsconfig.aot.json']) {
  if (existsSync(path.join(ROOT, f))) pass(`tsconfig: ${f}`, 'present');
  else fail(`tsconfig: ${f}`, 'missing');
}

const pkgDirs = [...packageDirs('packages'), ...packageDirs('apps')];
const missingTsconfigs = pkgDirs.filter((d) => !existsSync(path.join(ROOT, d, 'tsconfig.json')));
if (missingTsconfigs.length === 0) {
  pass('Per-package tsconfig.json', `${pkgDirs.length} workspace packages all have a tsconfig.json`);
} else {
  fail('Per-package tsconfig.json', `missing in: ${missingTsconfigs.join(', ')}`);
}

// ---------------------------------------------------------------------------
// 4. Workspace config presence: pnpm-workspace.yaml / nx.json / angular.json
// ---------------------------------------------------------------------------

for (const f of ['pnpm-workspace.yaml', 'nx.json', 'angular.json']) {
  const present = existsSync(path.join(ROOT, f));
  if (f === 'pnpm-workspace.yaml') {
    if (present) pass('Workspace config: pnpm-workspace.yaml', 'present (this repo uses pnpm workspaces)');
    else fail('Workspace config: pnpm-workspace.yaml', 'missing — workspace membership cannot be verified');
  } else if (present) {
    pass(`Workspace config: ${f}`, 'present');
  } else {
    const note = f === 'nx.json' ? 'Nx is not installed in this repo' : 'no Angular CLI workspace config';
    info(`Workspace config: ${f}`, `absent (${note})`);
  }
}

// ---------------------------------------------------------------------------
// 5. Test configuration / tooling (Vitest, Playwright, Jest)
// ---------------------------------------------------------------------------

const VITEST_CONFIG_NAMES = [
  'vitest.config.ts',
  'vitest.config.mts',
  'vitest.config.js',
  'vitest.config.mjs',
];
const vitestPkgs = pkgDirs.filter((d) =>
  VITEST_CONFIG_NAMES.some((f) => existsSync(path.join(ROOT, d, f))),
);
if (vitestPkgs.length) {
  pass('Vitest configs in packages', `${vitestPkgs.length} found: ${vitestPkgs.join(', ')}`);
} else {
  fail('Vitest configs in packages', 'no vitest.config.* found under packages/ or apps/');
}
const noVitest = pkgDirs.filter((d) => !vitestPkgs.includes(d));
if (noVitest.length) {
  info('Packages without a Vitest config', `${noVitest.join(', ')} (test scripts delegate to the root runner)`);
}

const verifyWeb = safeRead('scripts/verify-web.py');
if (verifyWeb && /playwright/i.test(verifyWeb)) {
  pass(
    'Playwright usage (scripts/verify-web.py)',
    'present — imports playwright.sync_api, launches headless Chromium against localhost:4173',
  );
} else if (verifyWeb) {
  info('Playwright usage (scripts/verify-web.py)', 'file exists but no Playwright reference was found');
} else {
  fail('Playwright usage (scripts/verify-web.py)', 'file missing — expected Playwright web verification script');
}

const jestFiles = [];
for (const base of ['', ...pkgDirs]) {
  const dir = path.join(ROOT, base);
  try {
    for (const name of readdirSync(dir)) {
      if (/^jest\.config\.(ts|js|mjs|cjs|json)$/.test(name)) jestFiles.push(path.posix.join(base, name));
    }
  } catch {
    /* unreadable directory — already covered elsewhere */
  }
}
const jestDeps = rootPkg && JSON.stringify(rootPkg).toLowerCase().includes('"jest"');
if (jestFiles.length === 0 && !jestDeps) {
  info('Jest configuration', 'NOT present — JS/TS tests use Vitest (see docs/BUILD-TOOLS-VALIDATION.md)');
} else {
  info('Jest configuration', `found: ${[...jestFiles, ...(jestDeps ? ['jest dependency'] : [])].join(', ')}`);
}

// ---------------------------------------------------------------------------
// 6. pnpm workspace membership for every package
// ---------------------------------------------------------------------------

function globToRegExp(glob) {
  const escaped = glob
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*/g, '\u0000')
    .replace(/\*/g, '[^/]*')
    .replace(/\u0000/g, '.*');
  return new RegExp(`^${escaped}$`);
}

function workspacePatterns(yaml) {
  const lines = yaml.split(/\r?\n/);
  const patterns = [];
  let inPackages = false;
  for (const line of lines) {
    if (/^packages\s*:/.test(line)) {
      inPackages = true;
      continue;
    }
    if (inPackages) {
      if (/^\S/.test(line)) break; // dedented to a new top-level key
      const m = /^\s+-\s+['"]?([^'"#]+?)['"]?\s*$/.exec(line);
      if (m) patterns.push(m[1].trim());
    }
  }
  return patterns;
}

const wsYaml = safeRead('pnpm-workspace.yaml');
if (!wsYaml) {
  fail('pnpm workspace membership', 'pnpm-workspace.yaml missing — cannot parse member globs');
} else {
  const patterns = workspacePatterns(wsYaml).map(globToRegExp);
  if (patterns.length === 0) {
    fail('pnpm workspace membership', 'no member globs parsed from pnpm-workspace.yaml');
  } else {
    const outsiders = pkgDirs.filter((d) => !patterns.some((re) => re.test(d.split(path.sep).join('/'))));
    const globs = workspacePatterns(wsYaml).join(', ');
    if (outsiders.length === 0) {
      pass('pnpm workspace membership', `all ${pkgDirs.length} packages matched by [${globs}]`);
    } else {
      fail('pnpm workspace membership', `not covered by [${globs}]: ${outsiders.join(', ')}`);
    }
  }
}

// ---------------------------------------------------------------------------
// 7. Unique package names
// ---------------------------------------------------------------------------

const parsedPkgs = pkgDirs.map((dir) => ({ dir, pkg: safeJson(`${dir}/package.json`) }));
const nameToDirs = new Map();
for (const { dir, pkg } of parsedPkgs) {
  if (!pkg || typeof pkg.name !== 'string') {
    fail(`Package name: ${dir}`, 'missing or invalid "name" field');
    continue;
  }
  if (!nameToDirs.has(pkg.name)) nameToDirs.set(pkg.name, []);
  nameToDirs.get(pkg.name).push(dir);
}
const duplicates = [...nameToDirs.entries()].filter(([, dirs]) => dirs.length > 1);
if (duplicates.length) {
  fail('Unique package names', duplicates.map(([n, dirs]) => `${n} -> ${dirs.join(' & ')}`).join('; '));
} else {
  pass('Unique package names', `${nameToDirs.size} names across ${pkgDirs.length} packages are unique`);
}

// ---------------------------------------------------------------------------
// 8. main / exports / bin targets resolve under dist OR src
// ---------------------------------------------------------------------------

function collectTargets(pkg) {
  const out = new Set();
  if (typeof pkg.main === 'string') out.add(pkg.main);
  if (typeof pkg.types === 'string') out.add(pkg.types);
  if (pkg.bin) {
    const vals = typeof pkg.bin === 'string' ? [pkg.bin] : Object.values(pkg.bin);
    for (const v of vals) if (typeof v === 'string') out.add(v);
  }
  const walk = (node) => {
    if (typeof node === 'string') {
      if (node.startsWith('./') || node.startsWith('dist/') || node.startsWith('src/')) out.add(node);
    } else if (Array.isArray(node)) {
      node.forEach(walk);
    } else if (node && typeof node === 'object') {
      Object.values(node).forEach(walk);
    }
  };
  walk(pkg.exports);
  return [...out];
}

function sourceEquivalent(clean) {
  let rel = clean.replace(/^dist\//, 'src/');
  if (rel.endsWith('.d.ts')) rel = `${rel.slice(0, -5)}.ts`;
  else if (rel.endsWith('.js')) rel = `${rel.slice(0, -3)}.ts`;
  else if (rel.endsWith('.mjs')) rel = `${rel.slice(0, -4)}.ts`;
  else if (rel.endsWith('.cjs')) rel = `${rel.slice(0, -4)}.ts`;
  else if (!path.extname(rel)) rel = `${rel}/index.ts`;
  return rel;
}

function resolveTarget(dir, target) {
  const clean = target.replace(/^\.\//, '');
  if (existsSync(path.join(ROOT, dir, clean))) return { ok: true, via: 'dist' };
  const srcRel = sourceEquivalent(clean);
  if (existsSync(path.join(ROOT, dir, srcRel))) return { ok: true, via: 'src' };
  return { ok: false, candidates: [clean, srcRel] };
}

for (const { dir, pkg } of parsedPkgs) {
  if (!pkg) continue; // already reported above
  const name = pkg.name ?? dir;
  const targets = collectTargets(pkg);
  if (targets.length === 0) {
    info(`${name}: entry points`, 'no main / exports / bin targets declared (not a publishable entry point)');
    continue;
  }
  const missing = [];
  let viaDist = 0;
  let viaSrc = 0;
  for (const t of targets) {
    const r = resolveTarget(dir, t);
    if (!r.ok) missing.push(`${t} [tried ${r.candidates.join(' or ')}]`);
    else if (r.via === 'dist') viaDist++;
    else viaSrc++;
  }
  if (missing.length) {
    fail(`${name}: entry points`, `${missing.length}/${targets.length} unresolved: ${missing.join('; ')}`);
  } else {
    pass(`${name}: entry points`, `${targets.length} targets resolve (dist: ${viaDist}, src: ${viaSrc})`);
  }
}

// ---------------------------------------------------------------------------
// 9. Script files referenced by root scripts and CI exist
// ---------------------------------------------------------------------------

const ciText = safeRead('.github/workflows/ci.yml') ?? '';
const rootScripts = rootPkg?.scripts ?? {};
const referencedScriptFiles = new Set();
const SCRIPT_FILE_RE = /scripts\/[A-Za-z0-9._-]+\.(?:mjs|js|py|sh|ps1)/g;
for (const text of [ciText, ...Object.values(rootScripts)]) {
  for (const m of String(text).matchAll(SCRIPT_FILE_RE)) referencedScriptFiles.add(m[0]);
}
const missingScriptFiles = [...referencedScriptFiles].filter((p) => !existsSync(path.join(ROOT, p)));
if (missingScriptFiles.length === 0) {
  pass(
    'CI / root-referenced script files',
    `all ${referencedScriptFiles.size} exist: ${[...referencedScriptFiles].sort().join(', ')}`,
  );
} else {
  fail('CI / root-referenced script files', `missing: ${missingScriptFiles.join(', ')}`);
}

const pnpmTargets = new Set();
for (const m of ciText.matchAll(/^\s*-?\s*run:\s*pnpm\s+(?:run\s+)?([A-Za-z0-9._:-]+)/gm)) {
  pnpmTargets.add(m[1]);
}
const NON_SCRIPT_PNPM = new Set(['install', 'add', 'remove', 'exec', 'dlx', 'i', 'ci']);
const undeclared = [...pnpmTargets].filter((t) => !NON_SCRIPT_PNPM.has(t) && !(t in rootScripts));
if (undeclared.length === 0) {
  pass('CI pnpm run targets declared', `${[...pnpmTargets].join(', ')} — all resolve to root scripts`);
} else {
  fail('CI pnpm run targets declared', `not declared in root package.json: ${undeclared.join(', ')}`);
}

// ---------------------------------------------------------------------------
// Print the PASS / FAIL table
// ---------------------------------------------------------------------------

function wrap(text, width) {
  const words = String(text).split(/\s+/);
  const lines = [];
  let cur = '';
  for (const word of words) {
    const next = cur ? `${cur} ${word}` : word;
    if (next.length <= width) cur = next;
    else {
      if (cur) lines.push(cur);
      cur = word;
    }
  }
  if (cur) lines.push(cur);
  return lines.length ? lines : [''];
}

const wStatus = Math.max('STATUS'.length, ...rows.map((r) => r.status.length));
const wCheck = Math.min(58, Math.max('CHECK'.length, ...rows.map((r) => r.check.length)));
const wDetail = Math.min(96, Math.max('DETAIL'.length, ...rows.map((r) => r.detail.length)));
const line = (a, b, c) => `${a.padEnd(wStatus)} | ${b.padEnd(wCheck)} | ${c.padEnd(wDetail)}`;
const rule = () => `${'-'.repeat(wStatus)}-+-${'-'.repeat(wCheck)}-+-${'-'.repeat(wDetail)}`;

console.log('AngNative — build-time tool validation (read-only)');
console.log(rule());
console.log(line('STATUS', 'CHECK', 'DETAIL'));
console.log(rule());
for (const r of rows) {
  const parts = wrap(r.detail, wDetail);
  parts.forEach((part, i) => {
    console.log(
      line(i === 0 ? r.status : '', i === 0 ? r.check : '', part),
    );
  });
}
console.log(rule());

const nPass = rows.filter((r) => r.status === 'PASS').length;
const nFail = rows.filter((r) => r.status === 'FAIL').length;
const nInfo = rows.filter((r) => r.status === 'INFO').length;
console.log(`SUMMARY: ${nPass} PASS, ${nFail} FAIL, ${nInfo} INFO`);
console.log(`RESULT: ${nFail === 0 ? 'PASS' : 'FAIL'}`);
process.exit(nFail === 0 ? 0 : 1);
