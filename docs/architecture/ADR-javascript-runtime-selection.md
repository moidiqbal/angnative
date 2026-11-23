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
- **Measured locally (post-change):** flagship universal 3-ABI release APK =
  **17,539,306 bytes (17.539 MB / 16.727 MiB)**; shipped asset
  `angnative-bundle.js` = 1,098,643 bytes (minified). Feature bundles remain
  2.0–2.6 MB test artifacts and are not shipped. Full breakdown:
  `docs/evidence/QUICKJS-NG-UPGRADE-AND-APK-SIZE.md`.
- Pre-change baseline APK = 23,312,447 bytes; baseline asset = 2,684,827 bytes.
- **Correction to earlier docs:** `rquickjs 0.5.1` vendors QuickJS with
  `VERSION = 2021-03-27` — it is NOT QuickJS-NG 2024 as previously claimed.
- **Implemented engine:** `rquickjs 0.14.0` vendors QuickJS-ng **0.16.2**
  (`QJS_VERSION_MAJOR 0 / MINOR 16 / PATCH 2`). QuickJS-ng v0.18.0 exists
  upstream but no published Rust binding vendors it; no v0.18 claim is made.
- No retained AngNative performance artifact (cold start, jank, peak memory) and
  no head-to-head measurements against V8/Hermes/RN/Flutter.

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
2. **Upgrade the vendored engine — implemented.** Moved from the 2021 snapshot
   to `rquickjs 0.14.0`, which vendors QuickJS-ng **0.16.2**. Android builds
   enable `bindgen` because `rquickjs-sys 0.14` ships no Android bindings. Moving
   to a binding that vendors QuickJS-ng v0.18+ is deferred until one is published.
   This is a security/correctness upgrade, not an architecture change.
3. **Minify the shipped bundle — implemented.** `minify: true` for
   `quickjs-bundle.js` only: 2,684,827 → 1,098,643 bytes. Feature bundles remain
   unminified test artifacts. Bytecode (`qjsc -b`) is NOT shipped: it needs a
   toolchain matched to the vendored engine plus a runtime loader.
4. **Enable Android R8 + resource shrinking — implemented** on the flagship
   release build, with explicit JNI keep rules. Lazy-load is NOT part of this
   round: feature bundles are not APK assets, so it has no APK-size effect.
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
- Risk: action #2 has landed for the shipped engine (QuickJS-ng 0.16.2), but the
  v0.18 security hardening is not yet reachable through a published Rust binding;
  revisit when one is published.

## Revisit criteria

Reconsider V8/JSC only when at least one is true:
- users require DevTools/source-map debugging in production, or
- a full browser API surface (routing via History, XHR, DOM libs) becomes a
  shipping requirement, or
- QuickJS-ng maintenance stalls for >12 months, or
- physical-device benchmarks show QuickJS bootstrap/jank below an agreed UX bar.

## References

- QuickJS-ng releases: https://github.com/quickjs-ng/quickjs/releases (v0.18.0 is
  upstream-only for this repo; the vendored engine is 0.16.2)
- QuickJS-ng security policy: SECURITY.md in the same repo
- rquickjs: https://github.com/DelSkayn/rquickjs
- Hermes: https://github.com/facebook/hermes
- deno_core: https://github.com/denoland/deno_core (merged into denoland/deno)
- Angular zoneless: https://angular.dev/guide/zoneless
- Local evidence: `docs/benchmarks/`, `docs/evidence/`, retained device screenshots
