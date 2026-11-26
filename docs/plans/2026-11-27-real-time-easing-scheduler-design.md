# Design — Real-time easing scheduler (priority #4)

**Date:** 2026-11-27
**Priority:** #4 after the local `v0.3.0` release and priorities #1–#3.
**Status:** validated with the user; not yet implemented.

## Objective

Lift the `Real-time animation easing fidelity` deferral with a scoped,
verifiable host-clock scheduler. Angular animations must be able to drive
intermediate native `SetStyle` mutations using elapsed time and a named easing
function, without changing the existing native mutation protocol or claiming a
60 FPS guarantee.

## Decisions

- **Dual-mode player.** The existing deterministic `0 → 0.5 → 1` player stays
  the default for `linear`/empty easing. A recognized non-linear easing name
  (`ease`, `ease-in`, `ease-out`, `ease-in-out`) opts into timed mode. This keeps
  all existing animation tests and MarketPulse evidence valid.
- **Dedicated clock.** `crates/angnative-runtime-bridge/src/lib.rs` registers a
  JS function `na_nowMs()` backed by a Rust `Instant` origin. `performance.now()`
  and rAF timestamps stay `0`, so legacy scheduling assumptions are untouched.
  No new JNI/FFI method and no Kotlin changes.
- **Frame driver.** Timed mode reschedules itself through
  `requestAnimationFrame` and reads `na_nowMs()` each frame. The host-drained
  microtask queue drives rAF as today; frame rate is **not** guaranteed.
- **Unchanged mutation path.** Player → `setStyle` → `CommandSetStyle` batch →
  bridge → `SetProps`. No protocol kind or decoder changes.

## Architecture

`packages/core/src/animation.ts` gains:

1. A tiny easing resolver. Named values map to standard cubic-bezier-style
   curves; unknown values resolve to linear and do not activate timed mode.
2. Timed activation only when easing is non-linear **and** both `na_nowMs` and
   `requestAnimationFrame` exist. Otherwise the current synchronous sampling
   path runs unchanged.
3. On `play()`: apply the first sample, record `start = na_nowMs()`, then
   reschedule frames via rAF. Per frame:
   `t = clamp((na_nowMs() - start) / duration, 0, 1)`,
   `eased = easing(t)`, interpolate every property at `eased` through the
   existing `setStyle` callback, and finish at `t >= 1`.
4. `pause()`, `cancel()`, `finish()`, `currentTime`, and the `finish` listener
   remain supported. A generation counter invalidates stale frame callbacks so
   cancel/replay cannot double-fire or mutate styles after cancel. Frame handles
   are flat numbers, preserving the no-QuickJS-object-cycle rule.

## Error handling

- Missing clock or rAF, unknown easing, or `duration <= 0` → deterministic path
  or immediate finish; never throw from scheduler setup.
- `finished` resolves at most once; listeners fire once.
- Cancel/pause clear the pending frame handle and bump the generation.

## Tests

- New `packages/core/src/animation.spec.ts` with a fake `na_nowMs` and a
  manually flushed rAF queue: `ease-in` over 100 ms at 0/25/50/75/100 ms must
  apply opacity `0, 0.0625, 0.25, 0.5625, 1`; also cover `ease-out`,
  `ease-in-out`, pause/cancel, missing-clock fallback and single `finish`.
- `packages/core/src/renderer.spec.ts`: one timed case asserting monotonic eased
  `SetStyle opacity` commands; the existing `0 → 0.5 → 1` test is unchanged.
- Extend `apps/demo-app/src/animations/main.animations.ts` with a second eased
  trigger and `__setEasedAnimationState()`. A new ignored Rust test in
  `crates/angnative-runtime-bridge/src/lib.rs` boots a fresh bridge, installs a
  controlled `na_nowMs` and a one-frame-at-a-time rAF queue before loading the
  same AOT bundle, flips the eased trigger, flushes frames, and asserts
  intermediate native-tree opacity values plus `start,done`. The new trigger
  starts `hidden`, so the historical bundle test remains valid.
- Re-run: `pnpm test`, `pnpm compile:aot:animations`, `pnpm bundle:framework`,
  the two ignored animation tests, `cargo test --workspace`, and
  `python scripts/check-ffi-declarations.py`.

## Evidence-first delivery

Record exact commands and observed outputs in
`docs/evidence/REAL-TIME-EASING.md` before updating `STATUS.md`,
`docs/UNSUPPORTED-FEATURES.md`, and `docs/architecture/issue-register.md`.
Commit locally; do not push. The two unrelated `Cargo.toml` modifications in the
working tree are preserved and excluded.

## Scope limits

- No 60 FPS, vsync, GPU, Hermes/V8, iOS, web, SSR or HMR claim.
- No new protocol kind, FFI declaration, JNI method or Kotlin capability.
- No change to deterministic linear animations or the socket-only MarketPulse
  feed.
