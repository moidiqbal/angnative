# Code Review — 2026-11-22

**Reviewer:** AI agent · **Scope:** current uncommitted working tree + last committed state
**Status of tree:** 7 modified tracked files + 4 untracked paths (runtime bridge, architecture docs). These are NOT from the v0.2.0 release commits — they appear to be in-progress work by you/teammate.

> **UPDATE (same day, after your explicit request):** All P0/P1 findings below were fixed. See **"Fix status"** at the end of this document. The table below records the *original* review state for the record; the current tree is green.

## Executive summary (original review state)

| Check | Original result | Current result |
|---|---|---|
| `pnpm test` | ✅ 219 passed | ✅ 219 passed |
| `cargo test --workspace` | ❌ **BUILD FAILS** — `angnative-host-engine` 7 compile errors | ✅ **129 passed, 0 failed** (6 ignored acceptance tests pass separately) |
| `cargo check --workspace` | ❌ same 7 errors | ✅ clean (no warnings) |
| Committed baseline (`fc8b6a2` + earlier) | ✅ was green (108 Rust tests) | ✅ still green |
| Current working tree | 🔴 broken; do not commit as-is | 🟢 green; fixes applied |

The in-progress "JS runtime wiring" is the right feature (it is our #1 gap from `tasklist.txt`), but the original implementation was **not compilable and had serious design defects**. The architecture docs you added (`docs/architecture/issue-register.md`) already identify 43 issues including these — this review confirmed them in code. All of the findings in this report have since been addressed; the remaining honest gaps are listed in **Fix status**.

---

## 🔴 P0 — must fix before any commit

### 1. Workspace does not compile (7 errors)
`crates/angnative-host-engine/src/runtime_controller.rs`:
- Line 48, 61, 86: `AngNativeRuntime` used but **not imported** (it lives in `crate::runtime`; `lib.rs` re-exports it as `crate::AngNativeRuntime`, but the module does not `use` it).
- Line 250, 264: `with_current_runtime_unchecked(...)` called but **never defined anywhere**.
- Line 168, 172: `self.runtime_in_thread_local` field **does not exist** on `RuntimeController` (fields: `host`, `state`, `config`, `runtime`).

Fix directions: add `use crate::{AngNativeRuntime, RuntimeConfig};` (RuntimeConfig already imported), define the missing helper or replace with a proper thread-local accessor, and either add the boolean field or remove the conditional.

### 2. `RuntimeController::new()` double-moves / duplicates the runtime (P0-01/P0-02 in your register)
```rust
// runtime_controller.rs:86-110
let mut runtime = AngNativeRuntime::new(...);
...
CURRENT_ANGULAR_RUNTIME.with(|cell| { *cell.borrow_mut() = Some(runtime); }); // MOVES runtime
Ok(Self { ..., runtime, })  // USE AFTER MOVE — will not compile even after fixing imports
```
`Option<AngNativeRuntime>` in thread-local + `RuntimeController.runtime` = two owners. Design comment says "single authoritative instance, no global singletons" but the code does the opposite. Pick one owner: either the controller owns it and the thread-local stores a raw pointer with strict lifetime, or the thread-local owns it and the controller holds a handle. Your `runtime-bridge` crate tries the raw-pointer approach — consolidate on ONE of the three (`RuntimeController`, `RuntimeBridge`, thread-local).

### 3. Three duplicate event dispatch paths (P0-04/P0-05 in your register)
- `Host::dispatch_event` now has: (a) FFI `event_callback`, (b) new thread-local `JS_RUNTIME_DISPATCH`, and (c) also calls both when both are present (bridge.rs:346-385) — so one click can invoke JS listeners **twice** with different listener-id semantics.
- `RuntimeBridge::dispatch_event` (runtime-bridge/src/lib.rs:74) calls `AngNativeRuntime::dispatch_event` directly.
- `RuntimeController::dispatch_event_from_callback` (line 259) is a third, unused path.
- Meanwhile `Host::add_event_listener` keeps ids in `event_listeners`, and `AngNativeRuntime` keeps its own `event_listeners` map — two registries that can never be populated together by the current shell code.

Consequence: pressing a button will either invoke nothing (if listener ids were registered through only one side) or invoke listeners twice. There must be exactly one dispatch pipeline.

### 4. Cross-thread QuickJS use is unsound (P0-03)
`CURRENT_BRIDGE_RUNTIME` stores `*mut AngNativeRuntime<QuickJsEngine>` in a thread-local, and `js_runtime_dispatch_callback` dereferences it (`&mut **runtime_ptr`) from a C callback. Comment says "same thread" but nothing enforces it — Android JNI callbacks are not guaranteed to arrive on the creating thread; `DispatchQueue.main.async` equivalents exist on iOS but not here. `rquickjs::Runtime` is not `Send`/`Sync`; undefined behaviour if this is violated. Enforce thread affinity explicitly (store thread id, assert in callback) or marshal events to the owning thread via a channel.

---

## 🟠 P1 — correctness / safety

### 5. `load_application()` injects bundle source into a string literal (runtime_controller.rs:195-207)
```rust
self.runtime.engine().eval(&format!("(async () => {{ ... {} ... }})()", bundle_js))
```
A bundle containing `*/`, backticks, or `${` will corrupt the generated JS → code injection/breakage. Use a proper `eval` of the bundle as-is, or set a global variable via the engine API and read it from JS. Also: the returned promise from `(async () => ...)()` is never awaited — module load errors are silently swallowed.

### 6. `drain_microtasks` LIFO + reentrancy hazard (runtime_controller.rs:220-227)
`while let Some(task) = tasks.pop()` executes microtasks in **reverse order** and while holding `borrow_mut()`. A microtask that calls `schedule_microtask` will panic (`already borrowed`) or deadlock. Use `std::mem::take(&mut *tasks)` then iterate in FIFO order.

### 7. FFI dispatch has no null/event-name guards and leaks on invalid input
`angnative_runtime_bridge_dispatch_event`/`nativeHostDispatchEvent`: `CStr::from_ptr` on C-supplied pointers is unsafe and unchecked; `CString::new(event_name).unwrap_or_default()` in bridge.rs turns an invalid string into an empty one and still dispatches. Validate UTF-8 and null explicitly, return 0 on error.

### 8. Double-free / ownership bug in Android `destroyHost()` (JniBridge.kt:119-125)
After the change, `destroyHost()` calls `nativeRuntimeBridgeDestroy(runtimeBridgeHandle)` which drops the `RuntimeBridge` (and hence the Host) — but the JNI ViewFactory context created in `initHost()` is registered against that host and is **never cleaned up**. In the original code `nativeHostDestroy` deleted the context + global ref. Now the global ref leaks on every activity recreation. Either move context cleanup into `angnative_runtime_bridge_destroy`, or keep an explicit `nativeHostDestroy` call before destroying the bridge (but not both).

### 9. Lifecycle state machine allows illegal `resume()` from `Running` (line 154-157)
`resume()` unconditionally transitions to `Running`; `VALID_TRANSITIONS` has no `(Running, Running)` so calling `resume()` twice returns an error. Guard with `if self.state != LifecycleState::Running`.

### 10. `#[no_mangle]` FFI functions catch panics around `RuntimeBridge::new()` but not around `dispatch_event` (runtime-bridge/src/lib.rs:155-214)
`dispatch_event` calls into QuickJS; any panic there unwinds across `extern "C"` → abort. Wrap all FFI entry points in `catch_unwind`.

---

## 🟡 P2 — maintainability / honesty

11. `docs/architecture/ADR-javascript-runtime-selection.md` says **Status: Pending (Awaiting Phase G Benchmarks)** — good, honest. Keep it that way until benchmarks actually exist. The code already committed to QuickJS; note that decision in the ADR or actually run the comparison.

12. `issue-register.md` has **duplicate heading `P0-06`** (Missing Angular Bootstrap / Missing QuickJS Microtask Scheduler) — numbering bug; second should be P0-07.

13. `RuntimeControllerConfig.max_heap_size` and `enable_inspector` fields are declared but **never used** — either wire them into `QuickJsEngine::new` / runtime config or remove to avoid false capability claims.

14. `quickjs_shims.rs` shims are very thin stubs (`appendChild` returns null, `setAttribute` does nothing, `dispatchEvent` always true). That is acceptable for a first pass, but Angular's renderer will not actually build a DOM — document that these shims cannot run a real Angular app yet (the architecture docs already say the bootstrap is missing).

15. `pnpm test` still passes because none of the new Rust code is reachable from JS tests. Once the Rust build is fixed, add tests that exercise the bridge end-to-end (register listener → dispatch event → listener invoked exactly once), otherwise the duplicate-dispatch bug will ship.

---

## ✅ What is good

- The architecture docs (`issue-register.md`, ADRs, engine comparison, NativeScript comparison) are unusually honest and specific — this is the right culture.
- The idea of a single `RuntimeBridge` owning Host + QuickJS + runtime is sound; the problems are in the half-finished execution of it.
- `set_js_runtime_dispatch_for_thread` with thread-local storage is a reasonable pattern *if* thread affinity is enforced.
- `LifecycleState` + `VALID_TRANSITIONS` is a clean design; just needs the edge cases fixed.
- Committed v0.2.0 baseline remains green — none of these defects are in the released code.

## Recommended order of work

1. Make it compile: imports + missing helper/field (P0-1).
2. Decide ONE owner for runtime and ONE event pipeline; delete the other two (P0-2/3).
3. Enforce thread affinity for QuickJS (P0-4).
4. Fix microtask FIFO + reentrancy, bundle injection, FFI guards (P1).
5. Fix Android bridge cleanup (P1-8).
6. Add end-to-end tests before calling this done.

Do not commit until `cargo test --workspace` is green and the "exactly once" event test passes.

---

## ✅ Fix status — 2026-11-22 (post-review)

After the user explicitly asked to fix all P0/P1 findings, the following changes were applied:

### P0 fixes

| # | Finding | Fix applied |
|---|---|---|
| 1 | Workspace does not compile (7 errors) | Rewrote `runtime_controller.rs` as a slim, lifecycle-only state machine. No missing types/helpers/fields remain. `cargo check --workspace` is clean. |
| 2 | Triple runtime ownership / use-after-move | `RuntimeController` no longer owns a runtime. **Single owner:** `RuntimeBridge` owns `Host + QuickJsEngine + AngNativeRuntime`. Removed the thread-local `CURRENT_ANGULAR_RUNTIME` ownership path. |
| 3 | Three duplicate event dispatch paths | `Host::dispatch_event` now invokes **exactly one** pipeline: canonical thread-local JS dispatch first (returns immediately if it handled the event), legacy FFI `event_callback` only as fallback. `RuntimeController` dispatch code was removed entirely. Single event path confirmed. |
| 4 | Cross-thread QuickJS unsound | `RuntimeBridge` stores the creating `ThreadId` and asserts it in `dispatch_event()` and `runtime_mut()`; cross-thread use panics instead of causing UB. Added `test_thread_affinity`. |
| 5 | Android `destroyHost()` JNI global-ref leak | `nativeRuntimeBridgeDestroy` now also accepts `hostHandle` and releases the `ViewFactoryContext` + JNI global ref from `g_contexts` (bridge dropped first so callbacks cannot fire with a dangling context). Kotlin `destroyHost()` passes the handle. |

### P1 fixes

| # | Finding | Fix applied |
|---|---|---|
| 6 | Bundle source injected into JS string literal | Bundle loading is no longer done through `format!` in the controller; shells use `RuntimeBridge::runtime_mut().eval(raw_source)` — no string-literal injection. |
| 7 | Microtasks LIFO + reentrancy panic | The microtask runtime was removed along with the controller rewrite; no borrow-held-while-running path remains. |
| 8 | FFI null/UTF-8 guards + panic safety | All bridge `extern "C"` entry points now validate null pointers and UTF-8, and wrap work in `catch_unwind`. Invalid input returns 0 instead of dispatching. |
| 9 | `resume()` double-call error | `resume()` is now idempotent (no-op when already `Running`). |

### Additional fix found during verification

- `install_dom_shims()` originally defined `window` *before* `console`/timers/`Event`/`fetch`/storage, causing a QuickJS `ScriptError` on undefined identifiers. Reordered so primitives are installed first and `window` last; native console backends are registered before the `console` shim.
- `window.self = window` / `window.globalThis = window` / `globalThis = window` as **stored** references were removed: cyclic object graphs cause a stack overflow in rquickjs 0.5.1 during GC/Drop (confirmed in debug and release, 1 MB and 8 MB stacks). They are now exposed as **getters** instead (no stored cycle), which the `test_self_is_a_safe_getter` test verifies.

### Gap coverage (follow-up on user request)

All *actionable* gaps from the original list were closed:

- **E2E "exactly once" test added** — `runtime-bridge::tests::test_event_dispatches_exactly_once_end_to_end` registers a JS listener, dispatches through both the direct bridge path and the `Host`/thread-local path, and asserts exactly one invocation per event (plus zero for wrong node/event).
- **DOM shims are now functional** — a flat, ID-based registry replaces the old stubs: `appendChild`/`removeChild`/`insertBefore`, `setAttribute`/`getAttribute`/`removeAttribute`/`hasAttribute`, real `parentNode`/`children`/`sibling`/`textContent` getters, `getElementById`/`querySelector(All)`, and working `addEventListener`/`dispatchEvent`. Covered by `test_dom_tree_operations` and `test_dom_text_content_and_insert_before`. The flat design is forced by the rquickjs cycle limitation: element objects hold only a numeric `_domId`, never object ↔ object links.
- **`window.self` restored** via getter (probe-tested safe); `window.window` and `window.globalThis` likewise.
- **Dangling-pointer bug found and fixed** — `RuntimeBridge` now holds the runtime in a `Box` so the address stored in the thread-local dispatch pointer stays stable when the bridge is moved into a `Box` by the C API (`angnative_runtime_bridge_create`). Without this, every JNI-dispatched event would have dereferenced a dangling pointer.
- **Bridge now installs the DOM shims** into its engine, so JS code actually has `window`/`document` available.
- **Android JNI + native build** — unused Kotlin `nativeHostCreate`/`nativeHostDestroy` declarations removed (bridge owns the Host; a standalone destroy would double-free). C++ legacy exports clearly marked. Added `scripts/check-ffi-declarations.py` (Kotlin↔C++ JNI arity, C++↔header, Swift↔header). The **real Android build exposed two more genuine defects**: the C header had no runtime-bridge declarations, and the Android build pipeline never built or linked the runtime-bridge crate (staged `.a` was also stale). Fixed by declaring the bridge ABI in the header, making `angnative-runtime-bridge` a `staticlib` (bundles host-ffi + engine + QuickJS into one archive), enabling Android `bindgen` for rquickjs-sys 0.5.1 (no shipped Android bindings) with NDK sysroot + VS LLVM libclang, and staging the unified archive for all three ABIs into both Android projects.

### Verification evidence

- `cargo test --workspace`: **129 passed, 0 failed** (6 acceptance tests ignored by default)
- `cargo test -p angnative-runtime-bridge -- --ignored`: **6 passed** (`dist/aot` bundles present)
- `cargo check --workspace`: clean, no warnings
- `pnpm test`: **219 passed**
- `python scripts/check-ffi-declarations.py`: OK (11 JNI methods, 9 Swift bindings, 19 header declarations)
- `pwsh -File scripts/build-android-native.ps1`: unified staticlib built and staged for arm64-v8a, armeabi-v7a, x86_64
- `showcases/ecosystem-core/gradlew assembleDebug`: **BUILD SUCCESSFUL** (all 3 ABIs)
- `scripts/build-android-local.ps1` (`platforms/android`): **BUILD SUCCESSFUL**

### Honest remaining gaps (NOT claimed as fixed)

- **Angular bootstrap in QuickJS now works** (issue P0-06 closed at the "component constructor + native tree" level). What was required:
  - `scripts/bundle-quickjs.mjs` (`pnpm bundle:quickjs`) produces a **self-contained IIFE** (Angular included, no ESM imports, `target: es2018` because QuickJS-ng cannot parse default ES2022 output) and explicitly imports `@angular/compiler` so Angular can JIT the decorated `@angnative/common` primitives shipped as plain tsc output.
  - `packages/platform/dist` was **stale** (still used `provideZoneChangeDetection` while source used `provideZonelessChangeDetection`) — rebuilding workspace packages (`pnpm --filter @angnative/{contract,core,platform} build`) was required before `compile:aot` / `bundle:quickjs`.
  - Runtime additions: raw `load_script` API (Rust + C ABI + JNI), `queueMicrotask` shim, native Promise-job drain (`Runtime::execute_pending_job`), polyfills for `Object.hasOwn`, `FinalizationRegistry`, `WeakRef`, `performance`, element-level `querySelector(All)`, kebab→camel style-name normalization, and custom component host tags (non-`na-*`) mapped to generic `na-view` containers.
  - **Acceptance test** `angular_bootstrap_runs_in_quickjs` (ignored by default; requires generated `dist/aot/quickjs-bundle.js`): loads the real AOT bundle, asserts no bootstrap rejection, and asserts the renderer emitted command batches containing `CreateNode`. **It passes**, with `[angnative demo] app bootstrapped on native renderer` logged from inside QuickJS.
  - **Full click round trip is now asserted** in the same acceptance test: the renderer registers its listener through `na_addEventListener`, the shim's `na_onEvent` routes Rust dispatch back to the Angular callback, `requestAnimationFrame` is driven from the host-drained microtask queue (Angular's zoneless rAF-vs-timeout scheduler needs this), and the test asserts a native event fires exactly one listener and produces a new command batch.
  - **Renderer → native pipeline is CLOSED** (previous highest-value gap): `RuntimeBridge` now registers `na_dispatchBatch(json)`, which calls `Host::process_batch_json` on the bridge-owned Host. The shim wires `__angnativeDispatchBatch` to it, so every renderer flush reaches the real Host `MutationEngine` / `OpEmitter` / `ViewFactory`. The acceptance test now asserts `__angnativeDispatchBatch` is a function, `host.engine().size() > 0`, native ops were emitted, and a click produces further native ops — no longer just `JSON.stringify(__angnativeBatches)`.
  - **On-device smoke test (emulator `AngNative34`, API 34)**: showcase APK installed and launched; logcat shows `AngNative runtime bridge created`, `View factory registered for host`, UI renders without crash. The showcase's ecosystem-core activity uses imperative Kotlin views, so the click round trip is verified in-process (Rust/QuickJS acceptance test), not by tapping this activity's UI.
  - **Phase 1 of Angular feature compatibility is done**: `@angular/forms` installed as a dev dependency, `pnpm bundle:forms` produces a self-contained forms bundle, and `angular_forms_work_in_quickjs` verifies `FormControl` / `FormGroup` / `Validators` values and validity inside QuickJS. `$localize` i18n shim (tagged templates + `loadTranslations`) is verified by `test_localize_i18n_translations`.
  - **Phase 2/3 (platform surface)**: real `@angular/router`, `@angular/animations` and `@angular/common/http` bundles are produced by `pnpm bundle:framework` and load in QuickJS. Browser shims added: `history`/`location`/`popstate` (routing), no-op `Element.animate()` driver (animations), and a host-wirable `fetch` (HttpClient). Five `--ignored` acceptance tests pass in total (bootstrap, forms, router, animations, http platform).
  - **Router — RESOLVED (F7)**: a real AOT router app (`apps/demo-app/src/router/main.router.ts`) bootstraps in QuickJS and navigates. Four root causes were fixed: (1) cycle-free `AbortController`/`AbortSignal` using a flat numeric-ID listener registry, (2) a real `URL` shim so Angular's serializer resolves `/about` instead of falling back to `/`, (3) `Renderer2.removeChild` now resolves the real parent from `child.parentId` when Angular passes null/synthetic root, (4) the synthetic root element (id 0) for null-parent calls. `angular_router_bootstraps_with_outlet` now asserts bootstrap, initial `HOME ROUTE`, imperative `navigateByUrl('/about')` → `ABOUT ROUTE`, location update and `NavigationEnd` lifecycle. It passes.
  - Still open (honest): full route-component swapping, Angular animation-engine integration, and DI-based `HttpClient.get()` (FetchBackend needs platform providers a bare EnvironmentInjector lacks — NG0201) are **not verified**. Template-driven `ngModel` bindings and AOT i18n markers in real app templates are also unverified. The 10 showcase UIs remain largely imperative reimplementations. This is not a claim of total Angular parity.
- **iOS/Mac cannot be built here** (Windows host, no Xcode). Swift bindings and the C header are statically cross-checked, and `scripts/build-ios-native.sh` is kept aligned, but no `swift build` or device verification was performed. Nothing here claims iOS verification.
- **No on-device runtime verification of Android** — the APK builds and links, but no emulator/device smoke test of the click → JS → Angular round trip was run.

