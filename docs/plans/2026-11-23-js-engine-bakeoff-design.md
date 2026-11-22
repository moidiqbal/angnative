# JS engine bake-off — design

**Date:** 2026-11-23
**Status:** Closed 2026-11-23 — QuickJS-NG retained by explicit decision;
Hermes/V8 implementation and the planned bake-off were not pursued. See
`docs/architecture/ADR-javascript-runtime-selection.md` for the accepted decision
and the measured 23,312,447-byte release APK (above the 20 MB target). This
document is retained as the pre-decision design, not as pending work.
**Scope:** Engine selection for the flagship AngNative Angular runtime. Rust host,
renderer, JNI/FFI, Taffy layout and the AOT bundle stay unchanged.

## Problem

The project goal is an exceptional, adoptable framework. Adoption depends on both
speed and binary size. The current runtime uses QuickJS-NG via `rquickjs`, but the
ADR (`docs/architecture/ADR-javascript-runtime-selection.md`) is still **Pending
(Awaiting Phase G Benchmarks)** and provisionally recommends V8/deno_core if
benchmarks confirm. Shipping on an unmeasured engine risks either a slow system
nobody adopts or a size regression nobody accepts.

## Locked constraints

| Constraint | Value |
| --- | --- |
| Total APK size | **≤ 20 MB**, measured on the **release universal 3-ABI APK** (`arm64-v8a`, `armeabi-v7a`, `x86_64`) |
| Device | Physical OnePlus Open `b6640c8f`, `CPH2551`, Android 16, `arm64-v8a` |
| WebView | None; zero-WebView native path only |
| Runtime claims | Android only; no iOS/macOS runtime claim (no Mac/Xcode here) |
| Comparative claims | None until head-to-head artifacts are retained |

## Candidates

1. **QuickJS-NG (`QuickJsEngine`) — baseline.** Already boots the real Angular AOT
   bundle and passes 8/8 OnePlus instrumentation tests. ADR estimates ~1.5 MB
   stripped engine; shims already implemented.
2. **Hermes — full challenger.** Implement shims, Android build and Angular AOT
   boot, then benchmark. ADR estimates ~3–4 MB engine; Rust bindings are partial.
3. **V8 / deno_core — measurement-only / expected excluded.** ADR estimates
   ~8–15 MB engine, which cannot fit the 20 MB universal-APK cap on top of the
   current ~19.2 MB self-measured showcase. Record the measured reason; do not
   build a full port this round.

## Architecture

The abstraction already exists:

- `crates/angnative-host-engine/src/engine.rs` defines the engine-neutral
  `JsEngine` trait.
- `crates/angnative-runtime-bridge/src/lib.rs` defines
  `RuntimeBridge<E: JsEngine = QuickJsEngine>`.
- `ReferenceEngine` already proves the abstraction with a second implementation.

Therefore no new trait is needed. Add `hermes` and `v8` modules behind Cargo
features, keeping `engine-quickjs` as the default so the shipping C ABI and
current device tests stay intact.

The real work is the shim surface, not the trait: QuickJS-specific shims live in
`RuntimeBridge::<QuickJsEngine>::new()`. Hermes must receive equivalent
`queueMicrotask`, timer and DOM/`EventTarget`-compatible shims or Angular AOT
will not boot. Ship each engine variant through the same benchmark harness:
same `dist/aot/quickjs-bundle.js`, same counter + 10k-grid workload, same device,
N≥5 warm runs.

JNI/FFI schema stays unchanged. `python scripts/check-ffi-declarations.py` and
the full gate run for every candidate.

## Decision gates

1. **Correctness.** 8 OnePlus instrumentation tests, ignored runtime-bridge
   suite, `pnpm test` and FFI declaration check. AOT boot failure or broken
   listener path eliminates the candidate.
2. **Size.** Release universal 3-ABI APK ≤ 20 MB. No exception: 20.1 MB fails.
3. **Speed.** Against QuickJS baseline on the same device and workload,
   p95 frame time must improve by **≥ 15%** and warm boot must not regress.
   Single-run differences are treated as noise.

If Hermes passes size and speed, it becomes the new default. If it passes size
but speed gain is noise-level, QuickJS is retained. If Hermes fails size, drop it
with a measured reason. V8 is expected to fail size and is not fully ported.

## Retained evidence

Store per candidate in `docs/evidence/engine-bakeoff/`:

- APK + SHA256
- `unzip -l` component breakdown (engine / Rust libs / JS bundle / resources)
- metrics JSON (cold boot, warm boot, p50/p95 frame, jank, RSS)
- logcat, instrumentation XML, UI XML, screenshot, device info
- exact build and benchmark commands

Final output: update the JS-runtime ADR from **Pending** to **Accepted/Rejected**
with numbers and retained artifacts, not written opinion.

## Estimate and out of scope

- Engine bake-off: **3–5 focused days**.
- Cross-framework A/B against React Native / Flutter / NativeScript: **1–2 weeks**,
  after the engine decision, not before.

Explicitly out of scope this round: CI `android-e2e`, zero-copy JNI, one-command
CLI, the other nine shells, DOM compatibility-doc cleanup, iOS runtime and
cross-framework performance claims.
