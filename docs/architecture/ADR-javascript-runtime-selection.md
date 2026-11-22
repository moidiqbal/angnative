# Architecture Decision Record: JavaScript Runtime Selection

## Status: **ACCEPTED — 2026-11-23** (supersedes "Pending Phase G benchmarks")

Phase C (Angular on QuickJS) is complete and verified on emulator and a physical
device. Phase G never produced an apples-to-apples V8 benchmark, so that
open-ended wait has been closed with the evidence now available. This ADR records
the actual decision, not a provisional preference.

## Context

AngNative requires an embedded JavaScript engine to execute Angular applications
natively, with **zero WebView**. Requirements:
1. Execute Angular 22+ AOT-compiled code
2. Support Angular zoneless change detection
3. Interface with the Rust host via FFI
4. Run on Android (ARM64), iOS (ARM64), Desktop (x64)
5. Meet binary size and startup budgets — **small build size is a primary user-facing goal**
6. No JIT on iOS
7. Active security maintenance and stable supply chain

## Decision drivers (ordered by priority for AngNative)

1. **Binary / APK delta** — users download the app; engine + bundles dominate
2. **iOS compatibility** — no JIT allowed
3. **Angular compatibility** — verified, not assumed
4. **Security maintenance / CVE cadence**
5. **Time-to-first-native-frame**
6. **Rust FFI ergonomics**
7. **GC pause / UI jank**
8. **Debug tooling**
9. **Cross-platform build complexity**

## Evidence gathered (2026-11-23)

### What is actually proven in this repo
- Angular 22 AOT bundle boots inside the embedded engine; renderer flushes reach
  `Host::process_batch_json`; native views are created; a real tap reaches an
  Angular listener and updates the UI (`Count: 0 → Count: 1`) — emulator
  (`emulator-5554`) and physical OnePlus (`b6640c8f`).
- Router, forms, i18n, animations engine, HttpClient platform surface, and RxJS
  7.8.2 all execute in the runtime (15 ignored acceptance tests pass).
- **Measured locally:** shipped `libangnative_runtime_bridge.so` = **1.56 MB**;
  flagship APK = **27.02 MB**; feature bundles = **2.0–2.6 MB each, unminified**.
- **Correction to earlier docs:** `rquickjs 0.5.1` vendors QuickJS with
  `VERSION = 2021-03-27` — it is NOT QuickJS-NG 2024 as previously claimed.

### Upstream state (authoritative GitHub release data)
| Engine | Latest / cadence | Binary |
|---|---|---|
| QuickJS-ng | **v0.18.0, 1 Nov 2026**; v0.16–0.18 contain dozens of UAF/OOB/leak/hash-hardening fixes, SECURITY.md, mimalloc, register regexp, GC-stress CI | **1.17–2.44 MB** release binaries |
| Hermes | **v0.13.0 for RN 0.75 (Aug 2024)** is the last tagged release; RN now builds Hermes from source → version coupling | ~3–4 MB (estimate) |
| V8 via deno_core | merged into denoland/deno; very active | **8–20 MB** (estimate); JIT banned on iOS |
| JSC | system framework on Apple | 0 MB on Apple; not cross-platform |
| MuJS / Duktape | active | 200–400 KB, but **ES5 only — cannot run Angular AOT** |

## Decision

**Retain the QuickJS family as the single primary engine.** Do NOT migrate to V8
or Hermes for the current product goals. Concretely:

1. **Keep rquickjs/QuickJS as the embedded engine.** Its 1.2–2.4 MB footprint is
   4–10× smaller than V8, it has no iOS JIT problem, and Angular compatibility is
   already proven with our shim layer — which neutralizes V8's main advantage.
2. **Upgrade the vendored engine.** Move from the 2021 snapshot to QuickJS-ng
   v0.18 (or a maintained binding that vendors it). This is a security and
   performance upgrade, not an architecture change.
3. **Ship bytecode / minified bundles.** Current bundles are 2.0–2.6 MB
   unminified; `minify: true` plus `qjsbc` bytecode are the largest available
   size wins without changing engines.
4. **Lazy-load feature bundles** (forms, router, animations, http) instead of
   bundling all into the boot path.
5. **Re-benchmark on a physical device** (startup ms, APK delta, jank, peak
   memory) before any future engine migration.

### Why not V8
8–20 MB binary (4–10× larger), iOS JIT ban forces interpreter mode, and the
"Angular runs natively" benefit is already achieved via shims. Revisit only if
users demand DevTools debugging or full browser API compatibility and accept
the size cost.

### Why not Hermes
Version-coupled to React Native releases (last standalone tag 2024), Rust
bindings (`hermes-rs`) incomplete, and ~3–4 MB for a capability we already have
at ~1.5 MB. Migration cost exceeds benefit.

### JSC as a future *complement*, not a replacement
On Apple platforms JSC costs zero binary size (system framework). Evaluate as a
platform-specific engine behind the engine-neutral `JsEngine` trait once macOS
work is possible. Keep Android/iOS on one engine for behavioral parity until
then.

## Consequences

- Positive: smallest possible APK, consistent iOS behavior, existing shim
  investment retained, clear upgrade path to a maintained engine.
- Negative: shim layer must be maintained as Angular evolves; no DevTools;
  interpreter-only throughput; GC pauses scale with heap.
- Risk: the current 2021 engine is a security liability until the upgrade in
  action #2 lands — treat that upgrade as high priority.

## Revisit criteria

Reconsider V8/JSC only when at least one is true:
- users require DevTools/source-map debugging in production, or
- a full browser API surface (routing via History, XHR, DOM libs) becomes a
  shipping requirement, or
- QuickJS-ng maintenance stalls for >12 months, or
- physical-device benchmarks show QuickJS bootstrap/jank below an agreed UX bar.

## References

- QuickJS-ng releases: https://github.com/quickjs-ng/quickjs/releases (v0.18.0)
- QuickJS-ng security policy: SECURITY.md in the same repo
- rquickjs: https://github.com/DelSkayn/rquickjs
- Hermes: https://github.com/facebook/hermes
- deno_core: https://github.com/denoland/deno_core (merged into denoland/deno)
- Angular zoneless: https://angular.dev/guide/zoneless
- Local evidence: `docs/benchmarks/`, `docs/evidence/`, retained device screenshots
