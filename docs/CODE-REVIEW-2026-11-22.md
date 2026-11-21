# Code Review — 2026-11-22

**Reviewer:** AI agent · **Scope:** current uncommitted working tree + last committed state
**Status of tree:** 7 modified tracked files + 4 untracked paths (runtime bridge, architecture docs). These are NOT from the v0.2.0 release commits — they appear to be in-progress work by you/teammate. **Nothing was modified during this review.**

## Executive summary

| Check | Result |
|---|---|
| `pnpm test` | ✅ 208 passed |
| `cargo test --workspace` | ❌ **BUILD FAILS** — `angnative-host-engine` 7 compile errors |
| `cargo check --workspace` | ❌ same 7 errors |
| Committed baseline (`fc8b6a2` + earlier) | ✅ was green (108 Rust tests) |
| Current working tree | 🔴 broken; do not commit as-is |

The in-progress "JS runtime wiring" is the right feature (it is our #1 gap from `tasklist.txt`), but the current implementation is **not compilable and has serious design defects**. The architecture docs you added (`docs/architecture/issue-register.md`) already identify 43 issues including these — this review confirms them in code.

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
