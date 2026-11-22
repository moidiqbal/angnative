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
  `catch_unwind`; a caught panic returns a null pointer / sentinel instead of
  unwinding across the FFI boundary. (The architect review flagged `get_host`
  and the dispatch callback as unwrapped; both were subsequently fixed.)
- **Owner-thread guard at the C ABI boundary.** Rust callers hit
  `assert_owner_thread()`, but Android release builds use `panic = "abort"`, so
  `catch_unwind` cannot catch that panic — it would kill the process. Every C
  ABI entry point that dereferences a bridge handle now checks the immutable
  `owner_thread` field with a non-panicking raw-pointer read first. Wrong-thread
  `dispatch_event` returns `DISPATCH_PANIC` (`u32::MAX`), `load_script` returns
  `2`, `get_host` returns `NULL`, and `destroy` is a no-op. No QuickJS code runs
  on the wrong thread. This is now verified on an Android 14 emulator with real
  wrong-thread JNI calls (2 tests / 0 failures); physical-device verification
  is still outstanding. See `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md`.
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
| `cargo test --workspace` | 135 passed, 10 ignored (9 Angular acceptance + 1 third-party spot-check) |
| `cargo test -p angnative-runtime-bridge -- --ignored` | 9 Angular acceptance tests + 1 third-party RxJS spot-check passed |
| Third-party library (native JS path) | RxJS 7.8.2 bundled unmodified via `pnpm bundle:third-party`; `third_party_rxjs_runs_unchanged_in_quickjs` asserts `of(1,2,3).pipe(map(x => x*2))` → `[2,4,6]`, `complete`, no error. **One DOM-free library only**; DOM-heavy UI libraries and third-party apps were not run. |
| `pnpm test` | 221 passed |
| Shared conformance fixtures (TS fake backend + Rust host) | 10 fixtures, including `angular-bootstrap-native` — the real Angular batch shape; Rust also runs it through `Host::process_batch_json` |
| Layout parity | Rust unit tests in `angnative-host-layout`; on-device ecosystem-core run produced a layout response for all 8 nodes. **No cross-backend layout parity fixture is retained** — treat as partial evidence |
| `python scripts/check-ffi-declarations.py` | 11 JNI methods, 14 Swift bindings (5 runtime bridge), 20 header declarations; iOS unified staticlib wiring OK |
| iOS static verification | `SwiftFFI.swift` binds `angnative_runtime_bridge_{create,destroy,dispatch_event,load_script,get_host}` with `UInt64`/`UInt32` signatures; `Package.swift` links `angnative_runtime_bridge`; `build-ios-native.sh` builds/stages that archive. No Swift compile, `swift test`, Simulator or device run on this host. |
| All 10 Android `assembleDebug` | SUCCESS observed during development; no CI Android job and no retained build log |
| On-device (API 34 emulator) | Angular bootstrap in QuickJS, 20 native nodes, `rc=0`; screenshot: `docs/assets/ecosystem-core-angular-in-rust-nodes20.png` |
| Wrong-thread JNI instrumentation (API 34 emulator) | `connectedDebugAndroidTest`: 2 tests, 0 failures; sentinels `-1` / `2` / NULL, bridge survives. Evidence: `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md` and `docs/evidence/android-thread-affinity-emulator.html`. Physical device not attached — no physical-device claim. |
| CI | `angular-runtime` job regenerates bundles and runs the ignored tests + FFI check; `android-e2e` job builds ecosystem-core and asserts the on-device node count. No build log artifact is uploaded/retained |

The ten ignored tests require regenerated `dist/aot/*` bundles, which are
gitignored. Reproduce locally with:
`pnpm --filter @angnative/core build && pnpm --filter @angnative/testing build &&
pnpm --filter @angnative/platform build` then
`pnpm compile:aot && pnpm compile:aot:router && pnpm compile:aot:forms &&
pnpm compile:aot:i18n && pnpm compile:aot:animations && pnpm bundle:quickjs &&
pnpm bundle:forms && pnpm bundle:framework && pnpm bundle:third-party`, then
`cargo test -p angnative-runtime-bridge -- --ignored`. Workspace packages
(`@angnative/contract`, `core`, `platform`) must be rebuilt first — a stale
`packages/platform/dist` was a real past source of incorrect behavior.

The legacy dispatch surface (`nativeHostDispatchEvent`, the FFI
`event_callback` fallback) is still shipped; it is not a second delivery path
but is relevant when reasoning about event handling.

---

## 8. Known, honest gaps (NOT claims)

1. **Full Router navigation — RESOLVED (F7)**: the AOT router app bootstraps,
   emits the outlet anchor, performs initial navigation (`/` -> `HOME ROUTE`),
   and executes imperative navigation (`navigateByUrl('/about')` -> `ABOUT ROUTE`).
   Root causes identified and fixed:
   - Added a cycle-free `AbortController`/`AbortSignal` shim using a flat, ID-keyed
     listener map (preventing rquickjs recursive GC stack overflow).
   - Added a standard `URL` shim so Angular's `isProtocolRelative` check in
     `DefaultUrlSerializer` does not fail and coerce all paths to `/`.
   - Fixed `Renderer2.removeChild` to resolve `actualParent` from `childEl.parentId`
     when Angular passes null/synthetic-root during outlet component deactivation.
   - Verified end-to-end via `angular_router_bootstraps_with_outlet` asserting
     both initial and destination route batches in QuickJS.
2. **Thread identity on native click callbacks — C ABI fixed and emulator-verified;
    physical device unverified.**
    Root cause: `RuntimeBridge::assert_owner_thread()` panics on a wrong-thread
    call; the C ABI's `catch_unwind` would normally convert that to a sentinel,
    but Android static libs are built with `cargo build --release` under
    `[profile.release] panic = "abort"`, so the process aborts instead. Fixed by
    a non-panicking owner-thread guard at every C ABI entry point that
    dereferences a bridge handle (raw read of the immutable `owner_thread`, so
    no aliasing reference is formed). Verified on the host by
    `test_c_abi_dispatch_from_non_owner_thread_returns_sentinel_without_panicking`
    and `test_thread_affinity`; the header contract is updated in
    `crates/angnative-host-ffi/include/angnative_host_ffi.h`. Verified on Android
    emulator by `RuntimeBridgeThreadAffinityTest.kt` (`connectedDebugAndroidTest`,
    2 tests / 0 failures). The first run failed because the staged `jniLibs`
    archive predated `fe8f08a`; rebuilding via `scripts/build-android-native.ps1`
    and rerunning produced the pass. **Not verified on a physical device:** at
    the time of the run `adb devices -l` showed only `emulator-5554`. Separately,
    JNI callbacks only `AttachCurrentThread` (no UI-thread marshalling), and
    `EcosystemCoreActivity` destroys the bridge immediately after loading the
    bundle. See `docs/EVENT-BRIDGE.md` and
    `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md`.
3. **Angular animation engine integration — RESOLVED (deterministic samples)**:
   `provideNativeAnimations()` wires Angular's real `AnimationEngine` and
   `AnimationRendererFactory` around `AngNativeRendererFactory2`. The AOT `fade`
   fixture (`apps/demo-app/src/animations/main.animations.ts`,
   `tsconfig.aot.animations.json`) uses explicit keyframes at offsets `0 / 0.5 / 1`.
   Five root causes were identified and fixed:
   - Angular creates the root component renderer with `hostElement = null`;
     stock `AnimationRendererFactory` then uses `BaseAnimationRenderer` and
     ignores `@trigger`. A flat synthetic root host is supplied.
   - Angular's engine requires `node.nodeType === 1`; `NativeElement.nodeType`
     is now numeric (`1 | 3 | 8`) with the Rust kind preserved as
     `nativeNodeType`.
   - `eraseStyles` writes `element.style[...] = ''`; a flat style mirror was
     added to `NativeElement` (the renderer's `styles` map stays authoritative).
   - Angular's orphan check calls `containsElement(document.body, element)`;
     the native animation driver now treats native elements as attached to the
     DOM-shim body so transitions are not downgraded to no-op players.
   - `animate('100ms')` with only state styles builds zero keyframes and a
     `NoopAnimationPlayer`; the fixture uses explicit keyframes.
   Evidence: ignored acceptance test
   `angular_animation_engine_interpolates_native_styles` asserts an intermediate
   native `SetStyle opacity: 0.5` between `start` and `done` callbacks and a
   final native `opacity: 1`. The player emits three deterministic samples via
   flat numeric-id global registries (no object cycles); real-time easing
   fidelity is **not verified**. The JIT-only test
   `packages/testing/src/integration.spec.ts` imports
   `@angular/platform-browser-dynamic` so Vitest can JIT-fallback-process the
   partially-compiled `@angular/animations/browser` declarations.
4. **DI-based `HttpClient.get()` — RESOLVED (NG0201)**: the real
   `@angular/common/http` bundle loads and a DI-resolved `HttpClient` performs
   `get('/api/x')` through `HttpHandler`/`FetchBackend`. The failure sequence was
   instrumented with Angular 22 dev-mode errors: first
   `InjectionToken HTTP_FETCH_MAX_RESPONSE_SIZE`, then `PendingTasks`, then
   `InjectionToken REQUESTS_CONTRIBUTE_TO_STABILITY`. Root cause:
   `createEnvironmentInjector()` creates an R3Injector with scope
   `{'environment'}`; its parent was a plain `Injector.create()` with no `root`
   scope, so every `providedIn: 'root'` token was unresolvable. The fix supplies
   `{provide: INJECTOR_SCOPE, useValue: 'root'}` from the same pnpm-resolved
   `@angular/core` instance, matching Angular's own `internalCreateApplication`
   root-injector pattern. `scripts/bundle-framework.mjs` exports
   `core.ɵINJECTOR_SCOPE` as `INJECTOR_SCOPE` and
   `http.ɵHTTP_FETCH_MAX_RESPONSE_SIZE` as `HTTP_FETCH_MAX_RESPONSE_SIZE` from
   that single instance, preserving token identity. The QuickJS shims now also
   provide UTF-8 `TextEncoder`/`TextDecoder` and `Response.body.getReader()`
   because `FetchBackend` decodes stream chunks rather than calling
   `response.json()`. Evidence: ignored acceptance test
   `angular_http_platform_loads_in_quickjs` asserts the native fetch body
   (`hello-from-native|GET`) and `HttpResponse.status === 200`. The converter in
   `quickjs.rs` is depth-limited because returning a cyclic RxJS `Subscription`
   previously overflowed rquickjs; the test returns a primitive and a new unit
   test `quickjs_engine_cyclic_value_does_not_overflow` covers truncation.
5. **iOS parity — bindings statically verified, execution NOT verified**:
   `SwiftFFI.swift` now wraps the legacy host **and** the five unified runtime
   bridge C ABI functions through `NativeRuntimeBridge`; `Package.swift` and
   `build-ios-native.sh` link `libangnative_runtime_bridge.a` (which bundles
   host-ffi + QuickJS). `check-ffi-declarations.py` verifies symbol presence,
   key signature types and build wiring on Windows. No Mac/Xcode is available,
   so Swift compile, `swift test`, Simulator/device runs and the runtime-bridge
   smoke checklist in `docs/IOS-BUILD.md` have not been executed. Nothing here
   claims iOS execution or UIKit Angular parity.
6. **AOT i18n markers — RESOLVED**: the AOT fixture
   `apps/demo-app/src/i18n/main.i18n.ts` uses `<na-text i18n>Hello {{ name }}!</na-text>`.
   `tsconfig.aot.i18n.json` emits `$localize`, `ɵɵi18n`, `ɵɵi18nExp` and
   `ɵɵi18nApply` into `dist/aot-i18n/i18n/main.i18n.js`. The first run hung
   because the `$localize` shim replaced the `\uFFFD0\uFFFD` marker in a
   `while (indexOf(marker) >= 0)` loop while AOT passed that same marker as the
   interpolation argument; it is now a single-pass `split(marker).join(...)`, so
   Angular's later `ɵɵi18nApply` performs the real substitution. The second run
   failed with `ReferenceError: Node is not defined` in `applyCreateOpCodes`;
   a flat `Node` constants shim (no object cycles) fixes it. Ignored acceptance
   test `angular_aot_i18n_translated_text_in_native_tree` registers the Angular
   i18n key before bundle evaluation and asserts the native tree contains the
   translated, interpolated text `Assalam-o-Alaikum AngNative!`.
7. **Template-driven `ngModel` — RESOLVED**: the AOT fixture
   `apps/demo-app/src/forms/main.forms.ts` uses `FormsModule` and
   `[ngModel]`/`(ngModelChange)` on a plain `<input>`. `canonicalElementTag()`
   now maps `input` to `na-text-input`, and a flat `navigator.userAgent` shim
   satisfies Angular's `DefaultValueAccessor` composition-mode check without
   object cycles. Ignored acceptance test
   `angular_template_driven_forms_round_trip_in_quickjs` asserts: (1) the JS
   model setter updates the native input `value` property and interpolated text
   node; (2) a Rust-dispatched native `input` event with
   `{"target":{"value":"typed-from-native"}}` fires exactly one Angular
   listener and updates the component model. Angular intentionally does not
   write the CVA value back for a user-originated event, so the test asserts the
   native label update rather than a duplicate input-property write.

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
- §7 now notes the `android-e2e` CI job and gitignored bundles.
- F7 router gap was subsequently **RESOLVED** (cycle-free AbortController, URL
  shim, renderer parent lookup); the acceptance test now asserts route swapping.

Residual caveat: build and on-device observations that were not retained as
repo artifacts are explicitly labelled as such. Treat this document as a
design map plus verified test evidence, not as an independent audit certificate.

