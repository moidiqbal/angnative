# Rust core size gate — CI wiring and budget source

**Date:** 2026-11-23
**Scope:** gate configuration only. **No local or CI size measurement result is
claimed by this document.**

## What was wired

- `.github/workflows/ci.yml` `size` job installs the pinned Rust toolchain
  (`nightly-2026-11-14`) and runs
  `node scripts/check-rust-size.mjs --host` alongside the existing JS fixture
  size budget. An explicit installed target may be used instead of `--host`.
- `scripts/check-rust-size.mjs` reads `rustCoreMaxBytesPerPlatform` from
  `packages/cli/src/budgets.json` (currently 1,048,576 bytes). The hardcoded
  1 MB default was removed; `--budget-bytes` remains an explicit override.
- The gate measures the stripped + LTO static-library artifacts for
  `angnative-host-core`, `angnative-host-layout`, `angnative-host-view` and
  `angnative-host-engine`.

## Explicit skips

On the Linux runner, Apple targets (`aarch64-apple-darwin`,
`x86_64-apple-darwin`, `aarch64-apple-ios`) and Windows
(`x86_64-pc-windows-msvc`) are skipped because those toolchains/targets are
unavailable there. Android cross-targets require an explicitly installed
target and are not measured by this job. These are documented skips, not
passing measurements.

## What is not claimed

- No measured byte count, pass/fail number, or "within budget" result is
  asserted here.
- No Apple, Windows, or Android cross-target measurement is claimed.
- No iOS/macOS runtime claim is made.

**Related register rows:** P2-11. See also `docs/R8-SIZE-BUDGET.md`.
