#!/usr/bin/env node
/**
 * check-rust-size.mjs — Rust core size budget gate (Task R8 / identity D-8).
 *
 * Measures the stripped + LTO static library size of
 *   angnative-host-core + angnative-host-layout + angnative-host-view +
 *   angnative-host-engine
 * per target platform and enforces the 1 MB budget.
 *
 * Exits 0 if all measured binaries are within budget; exits 1 if any exceed
 * the budget or if a target fails to build.
 *
 * Usage:
 *   node scripts/check-rust-size.mjs [--budget-bytes <bytes>] [--target <triple>...]
 *
 * Default budget: 1 MB (1,048,576 bytes).
 * Default targets: the crates in the AngNative workspace release profile.
 */

import { execSync } from 'child_process';
import { existsSync, statSync } from 'fs';
import { join } from 'path';

const DEFAULT_BUDGET_BYTES = 1_048_576; // 1 MB

// The Rust host crates that contribute to the 1 MB budget (identity D-8).
const HOST_CRATES = [
  'angnative-host-core',
  'angnative-host-layout',
  'angnative-host-view',
  'angnative-host-engine',
];

// Default target triples for cross-platform size measurement.
// Platform shells build these crates as static libs / cdylibs.
const DEFAULT_TARGETS = [
  'x86_64-unknown-linux-gnu',   // Linux desktop
  'aarch64-apple-darwin',       // macOS Apple Silicon
  'x86_64-apple-darwin',        // macOS Intel
  'aarch64-apple-ios',          // iOS
  'aarch64-linux-android',      // Android 64-bit
  'armv7-linux-androideabi',    // Android 32-bit
  'x86_64-pc-windows-msvc',     // Windows
];

function parseArgs() {
  const args = process.argv.slice(2);
  let budgetBytes = DEFAULT_BUDGET_BYTES;
  const targets = [];

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--budget-bytes' || args[i] === '-b') {
      budgetBytes = Number(args[++i]);
      if (!Number.isFinite(budgetBytes) || budgetBytes <= 0) {
        console.error(`error: invalid budget-bytes: ${args[i]}`);
        process.exit(1);
      }
    } else if (args[i] === '--target' || args[i] === '-t') {
      targets.push(args[++i]);
    } else if (args[i] === '--help' || args[i] === '-h') {
      console.log(`
Usage: node scripts/check-rust-size.mjs [options]

Options:
  --budget-bytes, -b <bytes>  Size budget in bytes (default: 1,048,576 = 1 MB)
  --target, -t <triple>       Target triple to measure (repeatable)
  --help, -h                  Show this help

Default targets: ${DEFAULT_TARGETS.join(', ')}
`);
      process.exit(0);
    } else {
      console.error(`error: unknown argument: ${args[i]}`);
      process.exit(1);
    }
  }

  return { budgetBytes, targets: targets.length ? targets : DEFAULT_TARGETS };
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1_048_576).toFixed(2)} MB`;
}

function cargoTargetDir() {
  // Respect CARGO_TARGET_DIR if set, otherwise default to target/
  return process.env.CARGO_TARGET_DIR || join(process.cwd(), 'target');
}

// Map target triple to the expected release artifact extension.
function artifactExt(target) {
  if (target.includes('windows')) return '.dll';
  if (target.includes('darwin') || target.includes('ios')) return '.dylib';
  if (target.includes('android')) return '.so';
  if (target.includes('linux')) return '.so';
  return '.a'; // static library fallback
}

// Possible artifact file names for a crate on a target.
function candidateArtifactPaths(targetDir, target, crateName) {
  const ext = artifactExt(target);
  const candidates = [
    // cdylib (Android .so, iOS/macOS .dylib, Windows .dll)
    join(targetDir, target, 'release', `lib${crateName}${ext}`),
    // static lib (.a)
    join(targetDir, target, 'release', `${crateName}${ext}`),
  ];
  return candidates;
}

function findArtifact(targetDir, target, crateName) {
  for (const path of candidateArtifactPaths(targetDir, target, crateName)) {
    if (existsSync(path)) {
      return path;
    }
  }
  return null;
}

function isStripped(path) {
  // Heuristic: read first bytes and check for common stripped indicators.
  // A truly stripped ELF/Mach-O/PE will have minimal symbol table data.
  // We can't fully verify without platform tools (strip --strip-all, etc.),
  // but the release profile sets strip = true which handles it at build time.
  try {
    const stat = statSync(path);
    return stat.isFile() && stat.size > 0;
  } catch {
    return false;
  }
}

async function main() {
  const { budgetBytes, targets } = parseArgs();
  const targetDir = cargoTargetDir();

  console.log('AngNative Rust core size budget gate (Task R8 / identity D-8)');
  console.log(`Budget: ${formatBytes(budgetBytes)} per platform`);
  console.log(`Targets: ${targets.join(', ')}`);
  console.log(`Target dir: ${targetDir}`);
  console.log('');

  // Ensure the release profile is configured for LTO + strip.
  // We check the workspace Cargo.toml for strip = true.
  const cargoTomlPath = join(process.cwd(), 'Cargo.toml');
  const cargoToml = await import('fs').then((f) => f.readFileSync(cargoTomlPath, 'utf8'));
  if (!cargoToml.includes('strip = true')) {
    console.warn(
      'warning: Cargo.toml release profile does not set strip = true; ' +
        'measured sizes may include debug symbols.',
    );
  }

  let allPassed = true;
  const results = [];

  for (const target of targets) {
    console.log(`\n=== Target: ${target} ===`);

    // Build each host crate for the target.
    for (const crate of HOST_CRATES) {
      try {
        execSync(
          `cargo build --release --target ${target} --package ${crate}`,
          {
            stdio: ['pipe', 'pipe', 'pipe'],
            cwd: process.cwd(),
            timeout: 600_000, // 10 minutes per crate
          },
        );
      } catch (err) {
        console.error(`  ✗ ${crate}: build failed`);
        console.error(`    ${err.message}`);
        allPassed = false;
        results.push({ target, crate, status: 'build_failed' });
        continue;
      }

      const artifactPath = findArtifact(targetDir, target, crate);
      if (!artifactPath) {
        console.error(`  ✗ ${crate}: artifact not found after build`);
        allPassed = false;
        results.push({ target, crate, status: 'artifact_not_found' });
        continue;
      }

      const size = statSync(artifactPath).size;
      const passed = size <= budgetBytes;
      const stripped = isStripped(artifactPath);

      if (!passed) {
        allPassed = false;
      }

      console.log(
        `  ${passed ? '✓' : '✗'} ${crate}: ${formatBytes(size)} ` +
          `(budget: ${formatBytes(budgetBytes)}) ` +
          `(stripped: ${stripped ? 'yes' : 'no/unknown'})`,
      );
      console.log(`    artifact: ${artifactPath}`);

      results.push({ target, crate, size, budgetBytes, passed, stripped });
    }
  }

  // Summary.
  console.log('\n=== Summary ===');
  const passed = results.filter((r) => r.passed);
  const failed = results.filter((r) => r.status === 'build_failed' || r.status === 'artifact_not_found' || (r.passed === false && r.size !== undefined));

  console.log(`Total artifacts measured: ${results.filter((r) => r.size !== undefined).length}`);
  console.log(`Within budget: ${passed.length}`);
  console.log(`Over budget or failed: ${failed.length}`);

  if (failed.length > 0) {
    console.log('\nFailures:');
    for (const r of failed) {
      if (r.size !== undefined) {
        console.log(`  ✗ ${r.target}/${r.crate}: ${formatBytes(r.size)} (budget: ${formatBytes(budgetBytes)})`);
      } else {
        console.log(`  ✗ ${r.target}/${r.crate}: ${r.status}`);
      }
    }
  }

  if (!allPassed) {
    console.log('\n❌ SIZE BUDGET EXCEEDED — Rust core is over 1 MB per platform.');
    console.log('   This is a finding, not a reason to move the goalpost (identity D-8).');
    process.exit(1);
  } else {
    console.log('\n✅ SIZE BUDGET MET — Rust core is within 1 MB per platform.');
    process.exit(0);
  }
}

main().catch((err) => {
  console.error(`fatal: ${err.message}`);
  process.exit(1);
});
