# AngNative Architectural Issue Register

## Audit Date: 2026-11-21
## Auditor: Principal Framework Architect

---

## Issue Classification

| Severity | Count |
|----------|-------|
| **P0 - Critical** | 8 |
| **P1 - High** | 12 |
| **P2 - Medium** | 15 |
| **P3 - Low** | 8 |
| **Total** | **43** |

---

## P0 - Critical Issues (Block Angular Execution)

### P0-01: Duplicate Host Instances
**Evidence**: Android JNI creates `Host#1` via `angnative_host_create()`, RuntimeBridge creates `Host#2` via `create_runtime_bridge_internal()`. No shared state.
**Files**: 
- `showcases/ecosystem-core/app/src/main/java/com/angnative/app/JniBridge.kt:62` (`nativeHostCreate()`)
- `crates/angnative-runtime-bridge/src/lib.rs:163` (`Host::new()`)
- `crates/angnative-host-ffi/src/bridge.rs:249` (`Host::new()`)
**Remediation**: Android must use `angnative_runtime_bridge_create()` instead of `angnative_host_create()`. Single Host per application.
**Acceptance Test**: Single Host instance verified via `angnative_host_node_count()` from both JNI and RuntimeBridge.

### P0-02: Unsafe Global Runtime Singleton
**Evidence**: `crates/angnative-runtime-bridge/src/lib.rs:27-65` uses `OnceLock<Arc<Mutex<Option<RuntimeBridge>>>>` with unsafe `transmute` to store bridge pointer.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs:27-65`
**Remediation**: Thread-local storage for QuickJS engine affinity. Opaque handle with explicit create/destroy.
**Acceptance Test**: No `unsafe` blocks in bridge creation. `MIRI` clean.

### P0-03: Cross-Thread QuickJS Access
**Evidence**: `QuickJsEngine` is not `Send/Sync` but wrapped in `Arc<Mutex<>>` at `crates/angnative-runtime-bridge/src/lib.rs:34`. C callback `js_runtime_dispatch_callback` accesses runtime from arbitrary thread.
**Files**: 
- `crates/angnative-runtime-bridge/src/lib.rs:34, 110-140`
- `crates/angnative-host-engine/src/quickjs.rs:27`
**Remediation**: Thread-local storage for runtime pointer. All QuickJS calls on owning thread only.
**Acceptance Test**: `std::thread::spawn` QuickJS call panics with clear message. No data races under `TSAN`.

### P0-04: Duplicate Event Dispatch Paths
**Evidence**: `Host::dispatch_event()` at `crates/angnative-host-ffi/src/bridge.rs:326-362` calls BOTH FFI callback AND JS runtime dispatch. Android `onNativePress` calls `nativeHostDispatchEvent` which goes to FFI callback path.
**Files**: 
- `crates/angnative-host-ffi/src/bridge.rs:326-362`
- `showcases/ecosystem-core/app/src/main/java/com/angnative/app/JniBridge.kt:230`
**Remediation**: Single authoritative event path - JS runtime dispatch ONLY. Remove FFI callback path.
**Acceptance Test**: Single `na_onEvent` invocation per native event. No duplicate Angular handler calls.

### P0-05: Disconnected Listener Registries
**Evidence**: Angular listeners registered via `Renderer2.listen()` → `RuntimeBridge::event_listeners`. Native events tracked in `Host::event_listeners`. Two separate registries.
**Files**: 
- `crates/angnative-host-engine/src/runtime.rs:30-33, 250-275`
- `crates/angnative-host-ffi/src/bridge.rs:245, 291-317`
**Remediation**: Single authoritative listener registry in RuntimeBridge. Host delegates to RuntimeBridge.
**Acceptance Test**: `addEventListener`/`removeEventListener` symmetry verified. No orphaned listeners.

### P0-06: Missing Angular Bootstrap in QuickJS
**Evidence**: No code loads Angular AOT bundle into QuickJS context. `bootstrapApplication` not wired to QuickJS runtime.
**Files**: 
- `packages/platform/src/bootstrap.ts` - not connected to QuickJS
- `crates/angnative-host-engine/src/runtime.rs:277-280` (`eval` only)
**Remediation**: Implement Angular platform adapter. Load AOT bundle via `RuntimeController`. Wire `bootstrapApplication`.
**Acceptance Test**: `bootstrapApplication(AppComponent)` executes in QuickJS. Component constructor runs.

### P0-06: Missing QuickJS Microtask Scheduler
**Evidence**: Angular zoneless change detection requires `queueMicrotask` implementation. QuickJS Promise jobs not drained.
**Files**: 
- `crates/angnative-host-engine/src/quickjs.rs` - no microtask drain
- `packages/platform/src/native-angular.ts` - expects zoneless
**Remediation**: Implement `drainMicrotasks()` in `AngNativeRuntime`. Call after each event dispatch.
**Acceptance Test**: `queueMicrotask` callback executes in same tick. Angular zoneless tick completes.

### P0-08: Android Uses Imperative Kotlin Batches
**Evidence**: `AngNativeView.kt:103-165` builds UI via hardcoded `BatchBuilder` commands instead of Angular renderer.
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/AngNativeView.kt:103-165`
**Remediation**: Remove `BatchBuilder`. Android shell only provides ViewFactory. Angular renderer drives UI.
**Acceptance Test**: No `BatchBuilder` usage. Angular renderer drives all UI mutations.

---

## P1 - High Issues (Degrade Correctness/Completeness)

### P1-01: FFI Ownership Unclear
**Evidence**: `AngNativeHost` opaque handle but no explicit ownership transfer. `angnative_host_destroy` takes raw pointer.
**Files**: `crates/angnative-host-ffi/src/ffi.rs:37-59`
**Remediation**: Explicit ownership model. `create` returns owned handle. `destroy` consumes handle. Null checks.

### P1-02: Null/Invalid Handle Handling
**Evidence**: FFI functions check `handle.is_null()` but not invalid-but-non-null pointers.
**Files**: `crates/angnative-host-ffi/src/ffi.rs`
**Remediation**: Handle validation. Generation counter or handle table.

### P1-03: No Handle Generation Counter
**Evidence**: Raw pointer cast as handle. No detection of use-after-free.
**Files**: `crates/angnative-host-ffi/src/bridge.rs:27-31`
**Remediation**: Handle table with generation counter. `angnative_host_validate()`.

### P1-04: Panic at FFI Boundary
**Evidence**: `catch_unwind` in FFI but panics propagate to `destroy`.
**Files**: `crates/angnative-host-ffi/src/ffi.rs:37-59`
**Remediation**: Panic containment. Log and return error code. No abort.

### P1-05: Callback Registration/Removal Asymmetry
**Evidence**: `add_event_listener` / `remove_event_listener` but no automatic cleanup on `destroy_host`.
**Files**: `crates/angnative-host-ffi/src/bridge.rs:291-317`
**Remediation**: `destroy_host` clears all listeners. Callback registry owns callbacks.

### P1-06: No Angular Platform Adapter
**Evidence**: `packages/platform/src/native-angular.ts` provides providers but no QuickJS integration.
**Files**: `packages/platform/src/native-angular.ts`, `packages/platform/src/bootstrap.ts`
**Remediation**: Implement `AngNativePlatform` with QuickJS runtime. Wire `bootstrapApplication`.

### P1-07: AOT Bundle Not Loaded
**Evidence**: `dist/aot/bundle.js` exists but never loaded into QuickJS.
**Files**: `scripts/compile-aot.mjs`, `crates/angnative-host-engine/src/runtime.rs`
**Remediation**: `RuntimeController.loadApplication(bundle_js)`. Module loading support.

### P1-08: No Module System in QuickJS
**Evidence**: `install_api()` only registers global functions. No `require`/`import`.
**Files**: `crates/angnative-host-engine/src/runtime.rs:55-245`
**Remediation**: Implement ES module loader. `require()` for Angular bundle.

### P1-09: Missing Angular Error Handling
**Evidence**: JS exceptions not propagated to Angular `ErrorHandler`.
**Files**: `crates/angnative-host-engine/src/runtime.rs:277-280`
**Remediation**: `try/catch` in `eval()`. Report to Angular `ErrorHandler` via host.

### P1-10: No Application Lifecycle
**Evidence**: No `CREATED` → `INITIALIZING` → `READY` → `RUNNING` → `DESTROYED` states.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: `RuntimeController` with explicit lifecycle. State machine.

### P1-11: No Runtime Metrics
**Evidence**: No memory, CPU, event latency tracking.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: `RuntimeMetrics` struct. Periodic collection.

### P1-12: Android ViewFactory Incomplete
**Evidence**: `NativeViewFactory.kt` missing `na-list`, `na-switch`, `na-activity-indicator`.
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/NativeViewFactory.kt:171-197`
**Remediation**: Implement all `na-*` view types. Match `@angnative/contract` elements.

---

## P2 - Medium Issues (Architectural Debt)

### P2-01: QuickJS Shim Maintenance
**Evidence**: `quickjs_shims.rs` growing. No automated validation.
**Files**: `crates/angnative-host-engine/src/quickjs_shims.rs`
**Remediation**: Automated shim validation. Minimal shim surface.

### P2-02: No QuickJS Bytecode Loading
**Evidence**: `qjsbc` bytecode not used. AOT bundle loaded as text.
**Files**: `crates/angnative-host-engine/src/runtime.rs:277-280`
**Remediation**: `RuntimeController.loadBytecode()`. Faster startup.

### P2-03: Android JNI Global Ref Leaks
**Evidence**: `ViewFactoryContext` global ref deleted in `destroyContext` but error paths may leak.
**Files**: `showcases/ecosystem-core/app/src/main/cpp/jni_bridge.cpp:49-55`
**Remediation**: RAII for JNI global refs. `ScopedGlobalRef`.

### P2-04: No Android UI Thread Enforcement
**Evidence**: ViewFactory callbacks may execute off UI thread.
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/NativeViewFactory.kt`
**Remediation**: `runOnUiThread` enforcement. Assert UI thread.

### P2-05: No Stress Tests
**Evidence**: Tests only cover happy paths.
**Files**: All test files
**Remediation**: Stress test suite. 10k events, 1k create/destroy cycles.

### P2-06: No Memory Growth Analysis
**Evidence**: No memory tracking in tests.
**Files**: All test files
**Remediation**: Memory tracking in integration tests.

### P2-07: Duplicate Test Infrastructure
**Evidence**: Conformance tests duplicated in Rust and TypeScript.
**Files**: `crates/angnative-host-view/tests/conformance_spec.rs`, `packages/testing/src/conformance.spec.ts`
**Remediation**: Single source of truth. Generate one from other.

### P2-08: No iOS Verification
**Evidence**: iOS shell code-complete but untested.
**Files**: `platforms/ios/`, `scripts/build-ios-native.sh`
**Remediation**: Mark iOS `UNVERIFIED`. Document blocker.

### P2-09: No Desktop Verification
**Evidence**: Desktop shell exists but no integration test.
**Files**: `crates/angnative-desktop/`
**Remediation**: Desktop integration test.

### P2-10: CLI Build Pipeline Gaps
**Evidence**: `compile:aot` works but no verification of output.
**Files**: `scripts/compile-aot.mjs`, `packages/cli/src/bundle.ts`
**Remediation**: Verify AOT output loads in QuickJS.

### P2-11: Size Budget Enforcement
**Evidence**: Size budget passes but Rust binary size not enforced per-target.
**Files**: `scripts/check-size-budget.mjs`
**Remediation**: Per-target size gates. CI enforcement.

### P2-12: No Cross-Engine Abstraction
**Evidence**: QuickJS types leak into runtime bridge.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs:197`
**Remediation**: Engine-independent interface. `JsRuntime` trait.

### P2-13: No Experimental Engine Adapters
**Evidence**: Hermes/V8 adapters not started.
**Files**: `crates/` - no `angnative-engine-hermes` or `angnative-engine-v8`
**Remediation**: Isolated proof-of-concept crates.

### P2-14: Benchmark Infrastructure Missing
**Evidence**: No automated benchmark runner.
**Files**: `scripts/` - no benchmark scripts
**Remediation**: Benchmark harness. Physical device automation.

### P2-15: Documentation Gaps
**Evidence**: Architecture docs created but not linked from code.
**Files**: `docs/architecture/`
**Remediation**: In-code references. `// See: docs/architecture/...`

---

## P3 - Low Issues (Polish)

### P3-01: Warning Cleanup
**Evidence**: `unused_doc_comments` on thread_local macros.
**Files**: Multiple
**Remediation**: Fix or allow.

### P3-02: Unused Imports
**Evidence**: `c_int` unused in runtime bridge.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs:24`
**Remediation**: Remove.

### P3-03: Dead Code
**Evidence**: `BatchBuilder` in Android shell (P0-08).
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/AngNativeView.kt:198-220`
**Remediation**: Remove after P0-08 fix.

### P3-04: Inconsistent Error Types
**Evidence**: `anyhow::Error` vs `EngineError` vs `String` errors.
**Files**: Multiple
**Remediation**: Unified error hierarchy.

### P3-05: Missing `Debug` Derives
**Evidence**: Some structs lack `Debug`.
**Files**: Multiple
**Remediation**: Add `#[derive(Debug)]`.

### P3-06: Inconsistent Naming
**Evidence**: `AngNativeRuntimeBridge` vs `RuntimeBridge` vs `runtime_bridge`.
**Files**: Multiple
**Remediation**: Consistent naming convention.

### P3-07: TODO Comments
**Evidence**: Multiple `// TODO` in code.
**Files**: Multiple
**Remediation**: Convert to issues or implement.

### P3-08: License Headers
**Evidence**: Some files missing license headers.
**Files**: Multiple
**Remediation**: Add MIT header.

---

## Tracking

| Issue | Status | Owner | Target Gate |
|-------|--------|-------|-------------|
| P0-01 | ✅ Fixed | - | A |
| P0-02 | ✅ Fixed | - | A |
| P0-03 | ✅ Fixed | - | A |
| P0-04 | ✅ Fixed | - | A |
| P0-05 | ✅ Fixed | - | A |
| P0-06 | 🔴 In Progress | - | C |
| P0-07 | 🔴 In Progress | - | C |
| P0-08 | 🔴 In Progress | - | D |
| P1-01 | ⬜ Pending | - | B |
| P1-02 | ⬜ Pending | - | B |
| P1-03 | ⬜ Pending | - | B |
| P1-04 | ⬜ Pending | - | B |
| P1-05 | ⬜ Pending | - | B |
| P1-06 | ⬜ Pending | - | C |
| P1-07 | ⬜ Pending | - | C |
| P1-08 | ⬜ Pending | - | C |
| P1-09 | ⬜ Pending | - | C |
| P1-10 | ⬜ Pending | - | B |
| P1-11 | ⬜ Pending | - | E |
| P1-12 | ⬜ Pending | - | D |
| P2-01 | ⬜ Pending | - | C |
| P2-02 | ⬜ Pending | - | C |
| P2-03 | ⬜ Pending | - | D |
| P2-04 | ⬜ Pending | - | D |
| P2-05 | ⬜ Pending | - | E |
| P2-06 | ⬜ Pending | - | E |
| P2-07 | ⬜ Pending | - | E |
| P2-08 | 🔴 Blocked | - | - |
| P2-09 | ⬜ Pending | - | - |
| P2-10 | ⬜ Pending | - | C |
| P2-11 | ⬜ Pending | - | E |
| P2-12 | ⬜ Pending | - | F |
| P2-13 | ⬜ Pending | - | F |
| P2-14 | ⬜ Pending | - | G |
| P2-15 | ✅ Done | - | A |
| P3-01 | ⬜ Pending | - | - |
| P3-02 | ⬜ Pending | - | - |
| P3-03 | ⬜ Pending | - | D |
| P3-04 | ⬜ Pending | - | - |
| P3-05 | ⬜ Pending | - | - |
| P3-06 | ⬜ Pending | - | - |
| P3-07 | ⬜ Pending | - | - |
| P3-08 | ⬜ Pending | - | - |

---

## Gate Completion Criteria

| Gate | Criteria | Status |
|------|----------|--------|
| **Gate A** | All P0 issues resolved | 🟡 In Progress |
| **Gate B** | Runtime lifecycle deterministic | ⬜ Pending |
| **Gate C** | Angular bootstrap + execution verified | ⬜ Pending |
| **Gate D** | Android E2E round-trip verified | ⬜ Pending |
| **Gate E** | All tests pass | ⬜ Pending |
| **Gate F** | Engine abstraction complete | ⬜ Pending |
| **Gate G** | Benchmarks complete | ⬜ Pending |

---

## Notes

- P0-01 through P0-05 addressed in runtime bridge refactor (thread-local, single Host, single event path)
- P0-06, P0-07, P0-08 are the current blocking issues for Gate C/D
- iOS (P2-08) remains BLOCKED pending macOS/Xcode access
- Documentation (P2-15) completed for research phase