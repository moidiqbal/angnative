import { execSync } from 'child_process';
import { existsSync } from 'fs';
import { join } from 'path';
import { pathToFileURL } from 'url';

/**
 * Result of a single doctor check.
 */
export interface DoctorCheck {
  name: string;
  ok: boolean;
  version?: string | null;
  required?: string;
  detail?: string;
}

/**
 * Full doctor result.
 */
export interface DoctorResult {
  ok: boolean;
  checks: DoctorCheck[];
}

/** Angular version floor (pinned). */
export const REQUIRED_ANGULAR = '22.2.5';
/** TypeScript version floor (pinned). */
export const REQUIRED_TYPESCRIPT = '6.0.3';
/** Node version floor (hard). */
export const REQUIRED_NODE_MAJOR = 22;
export const REQUIRED_NODE_MINOR = 12;

/**
 * Run a command and return its stdout, or throw if it fails.
 * Uses `where` on Windows and `which` on Unix — this fixes the M0 review
 * finding that the Windows path guard never matched (Windows has no `which`).
 */
function which(tool: string): string | null {
  const isWindows = process.platform === 'win32';
  const cmd = isWindows ? `where ${tool}` : `which ${tool}`;
  try {
    const out = execSync(cmd, { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
    return out.toString().trim().split(/\r?\n/)[0] || null;
  } catch {
    return null;
  }
}

/**
 * Run a version command and return the version string, or null.
 */
function versionOf(cmd: string): string | null {
  try {
    const out = execSync(cmd, { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
    return out.toString().trim();
  } catch {
    return null;
  }
}

/**
 * Check if a directory exists at the given env var path or common fallback.
 */
function dirExists(envVar: string, fallbacks: string[] = []): string | null {
  const fromEnv = process.env[envVar];
  if (fromEnv && existsSync(fromEnv)) return fromEnv;
  for (const fb of fallbacks) {
    if (existsSync(fb)) return fb;
  }
  return null;
}

/** Parse a semantic version like "v26.13.1" -> [26, 13, 1]. */
function parseSemver(v: string): [number, number, number] | null {
  const m = v.match(/^v?(\d+)\.(\d+)\.(\d+)/);
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

/**
 * Check Node version against the required floor.
 */
export function checkNode(): DoctorCheck {
  const raw = versionOf('node --version');
  const ok = raw !== null;
  let version: string | undefined;
  let meetsFloor = false;

  if (raw) {
    version = raw.replace(/^v/, '');
    const parsed = parseSemver(version);
    if (parsed) {
      meetsFloor =
        parsed[0] > REQUIRED_NODE_MAJOR ||
        (parsed[0] === REQUIRED_NODE_MAJOR && parsed[1] >= REQUIRED_NODE_MINOR);
    }
  }

  return {
    name: 'node',
    ok: ok && meetsFloor,
    version,
    required: `>=${REQUIRED_NODE_MAJOR}.${REQUIRED_NODE_MINOR}`,
    detail: meetsFloor ? undefined : `found ${version ?? 'unknown'}, need >=${REQUIRED_NODE_MAJOR}.${REQUIRED_NODE_MINOR}`,
  };
}

/**
 * Check Angular version (pinned exactly 22.2.5).
 */
export function checkAngular(): DoctorCheck {
  const raw = versionOf('npx ng version 2>/dev/null || node -e "console.log(require(\'@angular/core/package.json\').version)" 2>/dev/null');
  let version: string | undefined;
  let ok = false;

  if (raw) {
    version = raw.trim();
    ok = version === REQUIRED_ANGULAR;
  }

  return {
    name: '@angular/core',
    ok,
    version,
    required: REQUIRED_ANGULAR,
    detail: ok ? undefined : `found ${version ?? 'not installed'}, need exactly ${REQUIRED_ANGULAR}`,
  };
}

/**
 * Check TypeScript version (pinned exactly 6.0.3).
 */
export function checkTypeScript(): DoctorCheck {
  const raw = versionOf('tsc --version 2>/dev/null || npx tsc --version 2>/dev/null');
  let version: string | undefined;
  let ok = false;

  if (raw) {
    // tsc --version outputs "Version 6.0.3"
    version = raw.replace(/^Version\s*/i, '').trim();
    ok = version === REQUIRED_TYPESCRIPT;
  }

  return {
    name: 'typescript',
    ok,
    version,
    required: REQUIRED_TYPESCRIPT,
    detail: ok ? undefined : `found ${version ?? 'not installed'}, need exactly ${REQUIRED_TYPESCRIPT}`,
  };
}

/**
 * Check Rust toolchain (cargo + rustc).
 */
export function checkRust(): DoctorCheck {
  const cargo = which('cargo');
  const rustc = which('rustc');
  const ok = cargo !== null && rustc !== null;
  const version = versionOf('cargo --version');

  return {
    name: 'rust',
    ok,
    version: version ?? (cargo ? 'cargo found at ' + cargo : undefined),
    required: 'cargo + rustc',
    detail: ok ? undefined : 'cargo and/or rustc not found on PATH',
  };
}

/**
 * Check Android SDK (ANDROID_HOME + adb).
 */
export function checkAndroid(): DoctorCheck {
  const sdkHome = dirExists('ANDROID_HOME', [
    join(process.env.HOME || '', 'Library', 'Android', 'sdk'), // macOS
    join(process.env.LOCALAPPDATA || '', 'Android', 'Sdk'), // Windows
  ]);
  const adb = which('adb');
  const ok = sdkHome !== null && adb !== null;

  return {
    name: 'android-sdk',
    ok,
    version: sdkHome,
    required: 'ANDROID_HOME + adb',
    detail: ok
      ? undefined
      : `ANDROID_HOME=${sdkHome ?? 'not set'}, adb=${adb ? 'found' : 'not found'}`,
  };
}

/**
 * Check Android NDK.
 */
export function checkNdk(): DoctorCheck {
  const ndkHome = dirExists('ANDROID_NDK_HOME', [
    join(process.env.ANDROID_HOME || '', 'ndk-bundle'),
    join(process.env.ANDROID_HOME || '', 'ndk', '*'),
  ]);
  const ok = ndkHome !== null;

  return {
    name: 'android-ndk',
    ok,
    version: ndkHome,
    required: 'ANDROID_NDK_HOME',
    detail: ok ? undefined : 'ANDROID_NDK_HOME not set or directory missing',
  };
}

/**
 * Check Xcode (macOS only; optional on Linux/Windows).
 */
export function checkXcode(): DoctorCheck {
  const isDarwin = process.platform === 'darwin';
  if (!isDarwin) {
    return {
      name: 'xcode',
      ok: true,
      version: 'n/a (non-macOS)',
      required: 'xcodebuild (macOS only)',
      detail: 'skipped on non-macOS platform',
    };
  }

  const xcodebuild = which('xcodebuild');
  const ok = xcodebuild !== null;
  const version = versionOf('xcodebuild -version');

  return {
    name: 'xcode',
    ok,
    version: version ?? (xcodebuild ? 'xcodebuild found at ' + xcodebuild : undefined),
    required: 'xcodebuild',
    detail: ok ? undefined : 'xcodebuild not found on PATH (install Xcode from the App Store)',
  };
}

/**
 * Run all doctor checks and return the combined result.
 */
export function runDoctor(): DoctorResult {
  const checks: DoctorCheck[] = [
    checkNode(),
    checkAngular(),
    checkTypeScript(),
    checkRust(),
    checkAndroid(),
    checkNdk(),
    checkXcode(),
  ];

  const ok = checks.every((c) => c.ok);
  return { ok, checks };
}

/**
 * CLI entry point: `angnative doctor`.
 * Prints a table of checks and exits 0 if all pass, 1 if any fail.
 */
export function doctorMain(): number {
  const result = runDoctor();

  const maxName = Math.max(...result.checks.map((c) => c.name.length), 0);
  console.log('AngNative doctor — toolchain check\n');

  for (const c of result.checks) {
    const pad = ' '.repeat(maxName - c.name.length + 2);
    const status = c.ok ? '✓' : '✗';
    const ver = c.version ? ` ${c.version}` : '';
    const req = c.required ? ` (need ${c.required})` : '';
    console.log(`  ${status} ${c.name}${pad}${ver}${req}`);
    if (c.detail && !c.ok) {
      console.log(`    ${c.detail}`);
    }
  }

  console.log(`\n${result.ok ? 'ALL CHECKS PASSED' : 'SOME CHECKS FAILED'}`);
  console.log(`DOCTOR EXIT=${result.ok ? 0 : 1}`);
  return result.ok ? 0 : 1;
}

const selfPath = process.argv[1];
if (selfPath && import.meta.url === pathToFileURL(selfPath).href) {
  process.exit(doctorMain());
}
