# Verified Architecture — Angular in Native QuickJS (2026-11-22)

**Audience:** framework architects, platform maintainers.
**Scope:** the *shipped-and-verified* state of the runtime pipeline after the
2026-11-22 fixes. This document describes what the tests and on-device runs
actually prove — not aspirations. Where something is not verified, it says so.

---

## 1. Constraints that shaped the design

| Constraint | Consequence |
|---|---|
| **Zero WebView** (hard project rule, D-5) | Angular must execute in an embedded QuickJS engine; no browser runtime. |
| Engine-neutral `JsEngine` trait | QuickJS-ng is the default; TS never learns which engine is embedded. |
| rquickjs 0.5.1 **cannot GC cyclic JS object graphs** (debug *and* release) | DOM is a flat ID registry; no `el.parent`/`parent.child` object links; `window.self` is a getter; Promises never resolve with self-referencing objects. |
| QuickJS is **not Send/Sync** | `RuntimeBridge` records the creating `ThreadId` and asserts it on every dispatch; both the runtime and Host are boxed so thread-local raw pointers stay stable across moves. |
| Multiple platform shells | One C ABI + one unified bridge, staged into every shell. |

---

## 2. Component map

```
                    ┌──────────────────────── TS (AOT bundle) ─────────────────────────┐
                    │  Angular 22 AOT component (ɵɵdefineComponent / template fns)     │
                    │        │                                   ▲                      │
                    │        │ renderer flush                    │ na_onEvent           │
                    │        ▼                                   │                      │
                    │  __angnativeDispatchBatch(json)      listener callback           │
                    │  AngNativeRenderer → CommandBatch JSON                           │
                    └────────┬───────────────────────────────────┬──────────────────────┘
                             │ raw script + JSON                 │ events
┌────────────────────────────▼───────────────────────────────────┴──────────────────────┐
│ RuntimeBridge  (crates/angnative-runtime-bridge) — SINGLE OWNER                        │
│   ├─ Box<AngNativeRuntime<QuickJsEngine>>   (JS engine + na_* API + DOM shims)          │
│   ├─ Box<Host>                              (MutationEngine + OpEmitter + ViewTree)     │
│   └─ owner_thread: ThreadId                 (affinity assertions)                      │
│                                                                                         │
│   na_dispatchBatch ──► Host::process_batch_json ──► MutationEngine / OpEmitter / VF    │
│   thread-local JS dispatch callback ──► AngNativeRuntime::dispatch_event                │
└───────────────┬───────────────────────────────┬────────────────────────────────────────┘
                │ C ABI                         │ C ABI
      ┌─────────▼──────────┐          ┌────────────────────┐
      │ 10 showcase shells │          │ platforms/android: │
      │ unified JNI bridge │          │ unified .a staged, │
      │                    │          │ Kotlin still legacy│
      └────────────────────┘          │ iOS: legacy host   │
                                      └────────────────────┘
```

---

## 3. The two pipelines that matter

### 3.1 Native mutation pipeline (closed and verified)

1. `Host::process_batch_json` is invoked from JS via `na_dispatchBatch(json)`.
2. The shim wires `globalThis.__angnativeDispatchBatch` to that function.
3. Angular's renderer flush handler publishes `CommandBatch` JSON through the
   hook; it reaches the **bridge-owned Host**, not a private mutation engine.
4. `Host` validates the protocol, applies mutations to its retained shadow tree,
   emits `PlatformOp`s, and forwards them to the registered `ViewFactory`.

**Evidence:** `angular_bootstrap_runs_in_quickjs` asserts
`typeof __angnativeDispatchBatch === 'function'`, `host.engine().size() > 0`,
and native ops emitted before and after a click. On-device (ecosystem-core,
API 34 emulator) the status label read `Angular in Rust: nodes=20 (load rc=0)`;
the retained screenshot is `docs/assets/ecosystem-core-angular-in-rust-nodes20.png`.

### 3.2 Event pipeline (exactly one delivery per event)

`Host::dispatch_event` keeps **two mechanisms**: a canonical thread-local JS
dispatch callback, and a legacy FFI `event_callback` used only as a fallback
when the JS path reports zero invocations. `nativeHostDispatchEvent` remains
declared in all showcase JNI bridges for legacy shells. What is verified is
**not** "only one code path exists" but **"one delivery per event"** —
`test_event_dispatches_exactly_once_end_to_end` asserts exactly one listener
invocation, never double-firing.

1. Native view click → JNI → `RuntimeBridge::dispatch_event`.
2. `AngNativeRuntime::dispatch_event` looks up listener ids and calls the JS
   global `na_onEvent(listenerId, payload)`.
3. The shim's `na_onEvent` forwards to `__angnativeDispatchListener`, installed
   by `AngNativeRenderer`, which invokes the Angular component callback.
4. `requestAnimationFrame` and microtasks are drained from the host so
   zoneless change detection ticks and emits new native mutations.

**Evidence:** `angular_bootstrap_runs_in_quickjs` asserts exactly one listener
fires per event and a new native batch follows.

---

## 4. Ownership and lifetime rules

- `RuntimeBridge` is the **only** owner of Host + engine + angular runtime.
- `RuntimeController` is lifecycle state only (no runtime, no events).
- Both owned objects are boxed; thread-local raw pointers target the boxes.
- `Drop` clears both thread-local pointers and the JS dispatch callback.
- All exported C ABI lifecycle/dispatch functions (`create`, `destroy`,
  `dispatch_event`, `load_script`, `get_host`) and the internal
  `js_runtime_dispatch_callback` validate their inputs and wrap work in
  `catch_unwind`; a panic returns a null pointer / 0 instead of unwinding across
  the FFI boundary. (The architect review flagged `get_host` and the dispatch
  callback as unwrapped; both were subsequently fixed.)
- Host `create`/`destroy` legacy JNI exports remain for ABI compatibility. The
  **10 unified showcases** do not declare them from Kotlin, but
  `platforms/android` still declares and calls `nativeHostCreate` /
  `nativeHostDestroy` (`platforms/android/.../JniBridge.kt:43-44,61,108`). A
  standalone host destroy must never be called on a bridge-owned handle
  (double-free).

---

## 5. Bundle and timer infrastructure

- `pnpm compile:aot` (ngc) → real Ivy output with `ɵɵdefineComponent`.
- `pnpm bundle:quickjs` → self-contained IIFE (`target: es2018`, Angular
  compiler explicitly imported) because QuickJS cannot parse default ES2022
  output and the CLI bundle leaves packages external.
- Companion bundles: `bundle:forms` (`@angular/forms`), `bundle:framework`
  (router, animations, http, router-app).
- Microtask/timer shims: `queueMicrotask`, rAF and `setTimeout`/`setInterval`
  are driven from the host-drained job queue; Angular's zoneless scheduler and
  RxJS `asyncScheduler` need them.
- Polyfills: `Object.hasOwn`, `FinalizationRegistry`, `WeakRef`, `performance`,
  `$localize` (i18n), `history`/`location`/`popstate`, `Element.animate`,
  host-wirable `fetch`.

---

## 6. Shell topology (verified 2026-11-22)

| Shell | Bridge | Bundle load | Verified |
|---|---|---|---|
| ecosystem-core | unified | **yes** (flagship) | on-device: `nodes=20`, `rc=0` (screenshot retained: `docs/assets/ecosystem-core-angular-in-rust-nodes20.png`) |
| other 9 showcases | unified | asset present, opt-in API; **not auto-loaded** | all 10 `assembleDebug` observed during development (no CI Android job); material launched on emulator and logged `runtime bridge created`, 8 nodes, layout response, no crash (log excerpt not retained as a repo artifact) |
| platforms/android | unified static library staged; Kotlin JNI still calls legacy `nativeHostCreate` | legacy imperative UI | `assembleDebug` OK |
| iOS | legacy host Swift bindings | none | build not possible here (no macOS/Xcode) |

The 10 active `app/build.gradle.kts` files have distinct `applicationId`s. Note
that the `profiles/legacy` and `profiles/latest` templates still set
`com.angnative.app` for every shell; profile-based local builds would collide.

The 9 concept-demo shells deliberately do **not** auto-load the Angular bundle:
Angular batches and their own imperative batches use overlapping node IDs and
crash `Host::process_batch_json`. They ship the asset and the `loadScript` API
for opt-in use only.

---

## 7. Verification matrix

| Check | Result |
|---|---|
| `cargo test --workspace` | 124 passed |
| `cargo test -p angnative-runtime-bridge -- --ignored` | 6 acceptance tests passed |
| `pnpm test` | 208 passed |
| `python scripts/check-ffi-declarations.py` | 12 JNI methods, 9 Swift bindings, 20 header declarations |
| All 10 Android `assembleDebug` | SUCCESS observed during development; no CI Android job and no retained build log |
| On-device (API 34 emulator) | Angular bootstrap in QuickJS, 20 native nodes, `rc=0`; screenshot: `docs/assets/ecosystem-core-angular-in-rust-nodes20.png` |
| CI | `angular-runtime` job regenerates bundles and runs the ignored tests + FFI check; it does **not** build Android |

The six acceptance tests require regenerated `dist/aot/*` bundles, which are
gitignored. Reproduce locally with:
`pnpm compile:aot && pnpm compile:aot:router && pnpm bundle:quickjs &&
pnpm bundle:forms && pnpm bundle:framework`, then
`cargo test -p angnative-runtime-bridge -- --ignored`. Workspace packages
(`@angnative/contract`, `core`, `platform`) must be rebuilt first — a stale
`packages/platform/dist` was a real past source of incorrect behavior.

The legacy dispatch surface (`nativeHostDispatchEvent`, the FFI
`event_callback` fallback) is still shipped; it is not a second delivery path
but is relevant when reasoning about event handling.

---

## 8. Known, honest gaps (NOT claims)

1. **Full Router navigation**: the AOT router app bootstraps and emits an outlet
   anchor. During development diagnostics the navigation promise remained
   pending and no route-component swap was observed; those diagnostics were not
   retained as a repo artifact, so the claim is limited to "not verified".
   Needs a fuller Angular event-loop / stability integration.
2. **Thread identity on native click callbacks**: load and node inspection ran
   on the UI thread; a dedicated cross-thread dispatch test on-device is not
   done. A cross-thread call panics by design (caught by the C shim → 0).
3. **Angular animation engine integration**: only the API + a no-op
   `Element.animate()` driver are verified, not the real engine.
4. **DI-based `HttpClient.get()`**: bundle and fetch surface verified; full DI
   request is blocked by `FetchBackend` platform-provider requirements (NG0201).
5. **iOS parity**: Swift bindings still wrap the legacy host; nothing here
   claims iOS execution.
6. Template-driven `ngModel` bindings and AOT i18n markers in real templates
   are unverified.

---

## 9. Review / sign-off

This document is to be independently verified against the code by an architect
reviewer. Findings and corrections are recorded in the review section below.

### Architect review (2026-11-22)

Independent reviewer verdict: **NEEDS CORRECTION** (core architecture accurate;
over-broad absolutes and under-evidenced claims). All reported findings were
applied:

- §3.2 now states "one delivery per event; canonical path + legacy fallback"
  rather than "one path".
- §4 panic-safety wording now excludes `get_host` and the internal JS callback;
  the `platforms/android` legacy Kotlin declarations are called out.
- §2/§6 now distinguish unified showcase shells from `platforms/android`
  (unified static library staged, but Kotlin still uses legacy host JNI).
- §6/§7 on-device numbers now reference the retained screenshot artifact; the
  material emulator observation is marked as not retained in-repo.
- §7 notes that no CI Android job exists and that bundles are gitignored.
- §8 router gap is limited to "not verified" without unrecorded specifics.

Residual caveat: build and on-device observations that were not retained as
repo artifacts are explicitly labelled as such. Treat this document as a
design map plus verified test evidence, not as an independent audit certificate.
