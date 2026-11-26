# Evidence — Real-time easing scheduler (priority #4)

**Date:** 2026-11-27
**Repo / worktree:** `C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing`
**Branch:** `feat/real-time-easing-scheduler` (base `aa8f2bc`)
**Design:** `docs/plans/2026-11-27-real-time-easing-scheduler-design.md`

## Claim under test

Angular animations can opt into a **host-clock easing scheduler**: a recognized
non-linear easing name (`ease`, `ease-in`, `ease-out`, `ease-in-out`) drives
intermediate native `SetStyle` mutations from a dedicated monotonic clock via
`requestAnimationFrame`. Linear/empty easing keeps the historical deterministic
`0 → 0.5 → 1` samples. No 60 FPS, vsync, GPU, JNI/FFI, protocol, or platform
claim is made.

---

## 1. Implementation surfaces

- `packages/core/src/animation.ts`: named easing functions; dual-mode player
  (deterministic / timed); `na_nowMs()` clock; rAF rescheduling; generation-
  guarded `cancel`/`pause`/`finish`/`currentTime`; deterministic fallback.
- `packages/core/src/animation.spec.ts` (new): 5 fake-clock/rAF unit tests.
- `packages/core/src/renderer.spec.ts`: one timed `SetStyle` sequence test.
- `crates/angnative-runtime-bridge/src/lib.rs`: registers `na_nowMs()` from a
  Rust `Instant` origin; new ignored AOT test
  `angular_timed_easing_drives_native_styles`.
- `apps/demo-app/src/animations/main.animations.ts`: `easedFade` trigger plus
  `__setEasedAnimationState()` / `__easedAnimationEvents()` /
  `__resetEasedAnimationEvents()` / `__flushEasedFrame()` harness globals.

`performance.now()` and rAF timestamps remain `0`; no new JNI method, Swift
binding, C header, or mutation protocol kind was added.

## 2. Artifact regeneration

The isolated worktree needed a clean install and ngc-built Angular libraries
before AOT (the primary repo's root `node_modules` links point back to itself):

```powershell
pnpm install --frozen-lockfile --offline
pnpm run build
pnpm --filter @angnative/core run build
pnpm --filter @angnative/common run build   # tsc only — lacked Ivy metadata
```

The `@angnative/common` d.ts initially lacked `import * as i0 from "@angular/core"`,
so AOT reported `NG2012: Component imports must be standalone...`. Rebuilding
that package with the Angular compiler fixed it:

```powershell
node node_modules/@angular/compiler-cli/bundles/src/bin/ngc.js -p packages/common/tsconfig.ngc.tmp.json
```

Observed afterwards:

```text
8 import * as i0 from "@angular/core";
```

(`tsconfig.ngc.tmp.json` was a temporary, deleted local file.)

Then, matching the CI sequence:

```powershell
pnpm run compile:aot
pnpm run compile:aot:router
pnpm run compile:aot:forms
pnpm run compile:aot:i18n
pnpm run compile:aot:animations
pnpm run compile:aot:marketpulse
pnpm run bundle:quickjs
pnpm run bundle:forms
pnpm run bundle:framework
pnpm run bundle:third-party
pnpm run bundle:marketpulse
```

Observed for the animation entry (all steps exit `0`):

```text
> node scripts/compile-aot.mjs tsconfig.aot.animations.json
[angnative aot] Angular compiler-cli: ...\node_modules\@angular\compiler-cli\bundles\src\bin\ngc.js
[angnative aot] compiling with tsconfig.aot.animations.json...
[angnative aot] AOT compilation complete (Ivy definitions verified).

> node scripts/bundle-framework.mjs
  dist\aot\animations-app-bundle.js  2.5mb ⚠️
Wrote ...\dist\aot\animations-app-bundle.js
```

Artifact hash:

```powershell
Get-FileHash -Algorithm SHA256 dist/aot/animations-app-bundle.js
```

```text
DA06702FAF1B59A913937D344B845FB64B98CAAF1B8770B34D94CDD5858653AE
```

## 3. JS/TS tests

Focused run:

```powershell
pnpm exec vitest run packages/core/src/animation.spec.ts packages/core/src/renderer.spec.ts
```

```text
✓ packages/core/src/animation.spec.ts (5 tests)
✓ packages/core/src/renderer.spec.ts (23 tests)

 Test Files  2 passed (2)
      Tests  28 passed (28)
   Duration  11.21s
```

Full suite:

```powershell
pnpm test
```

```text
✓ packages/core/src/animation.spec.ts (5 tests)
✓ packages/core/src/renderer.spec.ts (23 tests)
✓ scripts/check-size-budget.spec.ts (8 tests)
... 18 files total ...
 Test Files  18 passed (18)
      Tests  266 passed (266)
   Duration  3.58s
```

One transient worktree-only parse failure was diagnosed and fixed locally: the
CRLF checkout of `scripts/check-size-budget.mjs` leaves its `#!/usr/bin/env node`
shebang visible to Vite's SSR loader (`SyntaxError: Invalid or unexpected token`
at the importer's second import). Converting that file to LF (git-normalized;
no tracked content change) made the same spec pass `8/8`. It is unrelated to the
easing changes and was restored to checkout state before commit.

## 4. Ignored runtime-bridge acceptance tests

Commands and results (each exit `0`):

```powershell
cargo test -p angnative-runtime-bridge -- --ignored --exact tests::angular_animation_engine_interpolates_native_styles --nocapture
```

```text
running 1 test
test tests::angular_animation_engine_interpolates_native_styles ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 1.19s
```

```powershell
cargo test -p angnative-runtime-bridge -- --ignored --exact tests::angular_timed_easing_drives_native_styles --nocapture
```

```text
running 1 test
test tests::angular_timed_easing_drives_native_styles ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 1.10s
```

```powershell
cargo test -p angnative-runtime-bridge -- --ignored --exact tests::market_pulse_bundle_drives_candlestick_and_animation --nocapture
```

```text
running 1 test
test tests::market_pulse_bundle_drives_candlestick_and_animation ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 2.04s
```

The new test asserts the runtime bridge exposes `na_nowMs`, then drives
controlled frames at 25/50/75/100 ms through the real Angular animation
engine. The native `opacity` history ends at `0.125`, `0.5`, `0.875`, `1`
respectively (ease-in-out), Angular's `start,done` callbacks fire, and the
final native node style is `opacity: 1`. Angular writes its hidden-state start
style more than once, so assertions check the most recent value, not an exact
whole-history string.

## 5. Whole-repo gates

```powershell
cargo test --workspace
```

The workspace exited `0`. Cargo prints per-crate summaries rather than one
workspace total; summing the 22 `test result: ok` lines gives:

```text
TOTAL passed=154 failed=0 ignored=18 lines=22
```

The runtime-bridge line itself is:

```text
test result: ok. 11 passed; 0 failed; 18 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

(The 18 ignored tests include the new timed-easing test.)

```powershell
python scripts/check-ffi-declarations.py
```

```text
FFI declaration check OK: 36 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 29 header declarations; iOS unified staticlib wiring OK
FFI_EXIT=0
```

Web size gate after the core change (still JIT-free and under budget):

```powershell
pnpm run compile:aot:web
pnpm run bundle:aot:web
pnpm check:web-size
```

```text
[angnative web] AOT bundle written to ...\dist\web-demo\app.js
[angnative web] JIT compiler packages absent from metafile inputs
[angnative web] raw bundle: 355177 B
[angnative web] gzip bundle: 112461 B (budget 128000 B / 125.0 KB, headroom 15539 B)
[angnative web] forbidden packages: 0
[angnative web] unlinked partial declarations: 0
WEB GATE EXIT=0
```

## 6. Scope limits

- Frame cadence is host-drained rAF; no 60 FPS, vsync, GPU, or device timing
  guarantee is claimed.
- iOS, Hermes/V8, SSR, web animation parity, HMR, and bytecode loading remain
  out of scope.
- `performance.now()` and rAF timestamps remain `0`; only the dedicated
  `na_nowMs()` capability supplies elapsed time.
- Deterministic linear animations and the socket-only MarketPulse feed are
  unchanged (both acceptance tests still pass).
