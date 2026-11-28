# Design: engine stub, verification closure and WIP promotion (2026-11-27)

**Status:** Accepted by user after an independent three-architect panel review.
**Base:** `fb609ec` (pushed; `master` clean vs `origin/master`).
**Rule:** every claim needs a passing test or retained artifact; otherwise it says **not verified**.

## Context

`fb609ec` shipped web/native HMR, borrowed direct-buffer JNI and the ANB1 binary
protocol. Its evidence doc lists gaps: real-browser HMR timing, emulator
execution, JSON-vs-ANB1 throughput and iOS execution. The working tree also
carries unrelated pre-existing WIP. The user asked for: a pluggable engine
adapter, formal SSR/hydration and DOM-library deferrals, dependency updates,
closure of closable gaps, and promotion of the dirty work.

## Decisions

1. **Engine:** `JsEngine` is already compile-time generic (`RuntimeBridge<E>`,
   `ReferenceEngine`). Add only `StubEngine` behind default-off `engine-stub` as
   a contributor template. It is non-executing, `eval` returns
   `EngineError::Internal`, `engine_name()` is
   `"StubEngine (non-executing test double)"`, and a test forbids `Hermes`,
   `V8`, `verified` or `integrated` in that name. No Hermes/V8 binary, no
   dynamic plugin ABI, no runtime engine switch. ADR 0001 and Gate F
   ("not integrated") stay unchanged.
2. **Conformance:** `tests/engine_conformance.rs` has two explicitly named
   tiers — A (API shape: QuickJS, ReferenceEngine, StubEngine) and B
   (JS semantics: QuickJS only). CI additionally runs
   `cargo test -p angnative-host-engine --features engine-stub`; the default
   workspace run proves the stub is absent from the shipping path.
3. **SSR/hydration:** remain formally unsupported. Guards check the register
   row and absence of `@angular/platform-server` — no runtime code, no
   "SSR tested and failed" claim.
4. **DOM-library parity:** remains formally unsupported. Guards check the
   register row and absence of declared DOM-heavy libraries. RxJS stays
   explicitly labelled a DOM-free spot-check, not general compatibility.
5. **Verification:** Python Playwright (1.65.2, cached Chromium) drives a real
   `DevServer` with no `compileHook` and no injected client dependencies; claim
   is "web HMR client under a harness" unless product wiring is promoted.
   Emulator and physical runs retain full provenance (serials, APK hashes,
   commands, UTC, raw output). JSON-vs-ANB1 is a scoped measurement with raw
   samples and no superiority claim. iOS stays not verified (no Xcode on the
   Windows host). JNI transport uses `capacity()`, so benchmark buffers are
   exact-sized; pooled-buffer support is not verified.
6. **Dependencies:** Angular 22.3.0 across every manifest, toolchain pin,
   doctor check, fixture and lockfile. TypeScript 7, Vitest 5 and
   @types/node 26 are skipped as majors.
7. **WIP promotion:** dirty changes are fixed and split into coherent commits.
   Excluded: the three Taiga `.a` build outputs; the recursive `build.sh` and
   Windows-only `patch.exe` path are repaired before commit; Taffy docs are
   completed (README, compatibility evidence, new ADR 0002 rather than
   rewriting ADR 0001); `AngularCounterLayoutTest.kt` is committed only with
   retained instrumentation evidence; all `JniBridge.kt` shells move together.

## Test and evidence plan

- `cargo test -p angnative-host-engine --tests` and with `--features engine-stub`.
- `cargo test --workspace`; `cargo test -p angnative-runtime-bridge -- --ignored`;
  `pnpm test`; `python scripts/check-ffi-declarations.py`.
- Browser: `scripts/verify-hmr-browser.py` + generated fixture under
  `dist/hmr-browser-evidence/`; retained JSON timings, dev-server log, SSE
  transcript, screenshots, environment block; ≥10 updates with
  `domAt - t0 < 1000 ms` each.
- Devices: full provenance logs for `emulator-5554` and `b6640c8f`, identical
  APK hashes, exact test command, every attempt retained.
- Benchmark: host-level Rust measurement with precomputed JSON/ANB1 inputs and
  fresh Host per iteration; device JNI arm reported separately; p50/p95/max
  and raw samples retained under `docs/benchmarks/anb1-vs-json/`.
- Guards: new Rust/TS tests plus a short evidence doc per deferral.

## Explicitly not verified

iOS/macOS build/execution; Hermes/V8 integration; dynamic engine plugins;
SSR/hydration at runtime; unchanged DOM-library parity; ANB1 over browser SSE;
end-to-end zero-copy (only the input borrow); pooled/reused direct buffers;
production dev-server auto-reload; WebView.
