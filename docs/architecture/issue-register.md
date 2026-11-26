# AngNative Architectural Issue Register

## Audit Date: 2026-11-21
## Reconciled: 2026-11-23 (P1/P2/P3 closure round)
## Auditor: Principal Framework Architect

---

## Issue Classification

| Severity | Count |
|----------|-------|
| **P0 - Critical** | 8 |
| **P1 - High** | 12 |
| **P2 - Medium** | 16 |
| **P3 - Low** | 8 |
| **Total** | **44** |

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
**Remediation**: one delivery per event (canonical JS thread-local path + legacy FFI fallback). No duplicate Angular handler calls.
**Acceptance Test**: One `na_onEvent` invocation per native event under the canonical JS thread-local path, with the legacy FFI fallback used only where the canonical path is unavailable.

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

### P0-08: Android Uses Imperative Kotlin Batches — CLOSED (flagship-only scope)
**Evidence**: `EcosystemCoreActivity` keeps the unified runtime bridge alive, loads the real AOT bundle, and a native tap reaches a live Angular listener on `emulator-5554` and OnePlus `b6640c8f` (`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`).
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/EcosystemCoreActivity.kt`, `JniBridge.kt`, `ViewFactoryMainThreadTest.kt`.
**Scope decision**: `ecosystem-core` is the supported Angular shell. Its remaining concept sections and the other 9 showcases stay imperative native reimplementations by design (ADR 0003); this is not a claim of Angular parity.
**Acceptance Test**: `ViewFactoryMainThreadTest.kt` plus real-tap logcat/XML; no `BatchBuilder`-removal claim is made.

---

## P1 - High Issues (Degrade Correctness/Completeness)

### P1-01: FFI Ownership Unclear — CLOSED
**Evidence (retained)**: The supported `angnative-runtime-bridge` path now exposes a validated `u64` handle registry with generation; `create` returns an owned handle and `destroy` consumes it. Verified by `test_thread_affinity_and_validated_handles` (unit) and the ignored `stress_1000_create_destroy_cycles_do_not_leak_handles` (see `docs/evidence/RUNTIME-BRIDGE-GATE-B.md`). **Scope caveat:** the legacy raw-pointer `angnative_host_*` FFI entry points are not covered by these bridge tests; no new claim is made about them.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`, `docs/evidence/RUNTIME-BRIDGE-GATE-B.md`
**Remediation**: None for the supported bridge path; retained evidence above closes this row.

### P1-02: Null/Invalid Handle Handling — Gate B implemented
**Evidence**: Validated `u64` bridge-handle registry rejects null, stale and non-owned handles; generation counters detect use-after-free. Verified by `test_thread_affinity_and_validated_handles` and `test_lifecycle_transitions_are_deterministic` (see `docs/evidence/RUNTIME-BRIDGE-GATE-B.md`). Legacy FFI raw-pointer validation is outside that proof.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: Bridge-side validation retained; Gate B passed for the supported path.

### P1-03: No Handle Generation Counter — Gate B implemented
**Evidence**: The validated handle registry carries a generation; stale handles are rejected after destroy/recreate. Verified by `test_thread_affinity_and_validated_handles` and ignored `stress_1000_create_destroy_cycles_do_not_leak_handles` (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: Generation counter implemented on the bridge; Gate B passed.

### P1-04: Panic at FFI Boundary — CLOSED
**Evidence (retained)**: Errors are unified in `angnative-host-engine/src/errors.rs` (`AngNativeError`, `ErrorCode`, `RuntimeErrorInfo`, `RuntimeMetrics`) and surfaced through error accessors instead of panics across the boundary; observed by `test_metrics_error_handler_and_native_callback` (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`). **Scope caveat:** this is proof for the bridge error/metrics path, not a new claim about every legacy `angnative_host_*` entry point.
**Files**: `crates/angnative-host-engine/src/errors.rs`, `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None; retained evidence closes this row for the supported path.

### P1-05: Callback Registration/Removal Asymmetry — CLOSED
**Evidence (retained)**: Destroy now releases registered callbacks/listeners with the handle registry; verified by `test_lifecycle_transitions_are_deterministic` and ignored `stress_1000_create_destroy_cycles_do_not_leak_handles` (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`). **Scope caveat:** covers the bridge lifecycle, not a new claim about legacy FFI listener teardown.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None; retained evidence closes this row for the supported path.

### P1-06: No Angular Platform Adapter — CLOSED
**Evidence**: `provideNativeAngular()` in `packages/platform/src/native-angular.ts` supplies the native `RendererFactory2`; the Rust host loads the AOT bundle into QuickJS and the flagship shell keeps the bridge alive on-device. Verified by the ignored `angular_bootstrap_runs_in_quickjs` test and the flagship live-tap evidence (`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`).
**Files**: `packages/platform/src/native-angular.ts`, `crates/angnative-host-engine/src/quickjs.rs`, `showcases/ecosystem-core/app/src/main/java/com/angnative/app/EcosystemCoreActivity.kt`.
**Remediation**: None; the integration is verified on the supported path.

### P1-07: AOT Bundle Not Loaded — CLOSED
**Evidence**: Real AOT bundles are loaded and executed in QuickJS: ignored tests `angular_bootstrap_runs_in_quickjs`, `angular_router_bootstraps_with_outlet` and `angular_aot_i18n_translated_text_in_native_tree`; the flagship shell also loads a real AOT bundle on-device (live-tap evidence above).
**Files**: `crates/angnative-host-engine/src/quickjs.rs`, `scripts/compile-aot.mjs`, `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`.
**Remediation**: None; the earlier stale claim is superseded by passing tests and retained artifacts.

### P1-08: No Module System in QuickJS — DEFERRED (not a blocker)
**Evidence**: `install_api()` registers globals and has no `require`/`import`, but the verified AOT bundles are self-contained IIFEs (`docs/CODE-REVIEW-2026-11-22.md`), so no module loader is needed for the supported path. An ES module loader remains unimplemented and unverified.
**Files**: `crates/angnative-host-engine/src/runtime.rs:55-245`
**Remediation**: Optional future ES module loader; no current claim depends on it.

### P1-09: Missing Angular Error Handling — implemented (Angular ErrorHandler provider tests)
**Evidence**: Unified `AngNativeError`/`ErrorCode`/`RuntimeErrorInfo` (`crates/angnative-host-engine/src/errors.rs`) and bridge error accessors feed the Angular `ErrorHandler` provider; observed by `test_metrics_error_handler_and_native_callback` and the Angular ErrorHandler provider tests being added in this round.
**Files**: `crates/angnative-host-engine/src/errors.rs`, `crates/angnative-runtime-bridge/src/lib.rs`, Angular platform provider tests
**Remediation**: Provider tests are part of this round's closure; retained in the closing commit.

### P1-10: No Application Lifecycle — Gate B implemented
**Evidence**: `RuntimeController` in `crates/angnative-runtime-bridge/src/lib.rs` provides deterministic lifecycle transitions, verified by `test_lifecycle_transitions_are_deterministic` (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None; Gate B passed.

### P1-11: No Runtime Metrics — implemented
**Evidence**: `RuntimeMetrics` and metrics/error accessors exist in `crates/angnative-host-engine/src/errors.rs` and `crates/angnative-runtime-bridge/src/lib.rs`; verified by `test_metrics_error_handler_and_native_callback`.
**Files**: `crates/angnative-host-engine/src/errors.rs`, `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None for accessor-level metrics; no periodic-collection claim is made.

### P1-12: Android ViewFactory Incomplete — implemented (tagged custom conformance fixture + MarketPulse candlestick)
**Evidence**: Tagged custom primitives (`na-list`, `na-switch`, `na-activity-indicator`) are covered by the `native-primitives-tagged-custom` conformance fixture. The MarketPulse round adds `na-candlestick` → `Custom:na-candlestick`, a shared `attr:bars` JSON-string fixture, the Rust op-emitter assertion, and `CandlestickViewFactoryTest` (`OK (1 test)` on `emulator-5554`). The UI dump shows `content-desc="Custom:na-candlestick"`; full instrumentation after the two-asset split is `OK (9 tests)` (`docs/evidence/ANDROID-MARKETPULSE.md`).
**Files**: `platforms/android/app/src/main/java/com/angnative/app/NativeViewFactory.kt`, `CandlestickView.kt`, `packages/testing/fixtures/candlestick-bars-property.json`, conformance fixture
**Remediation**: Retained. The historical CoinGecko APK's physical `OK (9 tests)` run is not claimed for the final socket APK. The active socket-only build shows `Custom:na-candlestick` at `[168,826][1064,1134]` with a painted match-derived candle on `b6640c8f`, and post-change ABI instrumentation passed there (`docs/evidence/ANDROID-MARKETPULSE.md` section 11; `docs/assets/market-pulse-abi-oneplus.log`).

---

## P2 - Medium Issues (Architectural Debt)

### P2-01: QuickJS Shim Maintenance — implemented (tagged custom conformance fixture + MarketPulse fixture)
**Evidence**: The `native-primitives-tagged-custom` conformance fixture provides automated coverage for tagged custom primitives on the native path. The MarketPulse round adds a shared OHLC bars-string fixture used by TS, Rust and Android; the ignored `market_pulse_bundle_drives_candlestick_and_animation` test loads the real AOT bundle and asserts the `attr:bars` mutation, 200-row window and deterministic animation samples. The socket-only round expands the shim with `na_fetchAsyncStart`/`na_onFetchResult` and `na_timerSchedule`/`na_timerCancel`/`na_onTimerTick`; the owner-scoped Rust queue is drained by `angnative_runtime_bridge_take_host_tasks(handle)` with `fetch`, `timerSchedule` and `timerCancel` JSON tasks. `cargo test --workspace` totals 154 passed / 0 failed / 17 ignored; the ignored runtime-bridge suite reports 17 passed / 0 failed.
**Files**: `crates/angnative-host-engine/src/quickjs_shims.rs`, `crates/angnative-runtime-bridge/src/lib.rs`, `crates/angnative-host-ffi/include/angnative_host_ffi.h`, `platforms/android/app/src/main/java/com/angnative/app/HostCapabilities.kt`, conformance fixture, `packages/testing/fixtures/candlestick-bars-property.json`
**Remediation**: Fixtures retained; host-backed async fetch/timer capabilities are implemented and verified. No REST market-data path was added (see P2-16).

### P2-02: No QuickJS Bytecode Loading — DEFERRED (feasibility spike only)
**Evidence**: `rquickjs` 0.5.1 exposes dump-bytecode and module-bytecode traits behind feature flags, but no runtime bytecode loader exists in this repo. The supported path remains self-contained IIFE bundles loaded as text. Outcome: **deferred, not verified**; see `docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md`. Do not claim bytecode loading works.
**Files**: `crates/angnative-host-engine/src/runtime.rs`, `docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md`
**Remediation**: A future `RuntimeController.loadBytecode()` would require a real loader plus tests; none exists.

### P2-03: Android JNI Global Ref Leaks — implemented (tests added this round)
**Evidence**: RAII ownership, failure-path cleanup and `onDestroy` teardown are covered by the Android JNI/native-view-factory tests added in this round (including tagged `NativeViewFactory` cases); emulator instrumentation may be re-run separately.
**Files**: `showcases/ecosystem-core/app/src/main/cpp/jni_bridge.cpp`, Android instrumentation tests
**Remediation**: Retained. Post-change ABI instrumentation was re-run on `b6640c8f` (`BUILD SUCCESSFUL in 1m 13s`) after the JNI/ABI changes; see `docs/assets/market-pulse-abi-oneplus.log`.

### P2-04: No Android UI Thread Enforcement — CLOSED
**Evidence**: Every `JniBridge` native entry point marshals to `Looper.getMainLooper()` with one post/latch per call; `NativeViewSink` records `Looper.myLooper()` inside callbacks and `ViewFactoryMainThreadTest.kt` asserts a main looper on emulator and OnePlus (4 tests / 0 failures each).
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/JniBridge.kt`, `ViewFactoryMainThreadTest.kt`, `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`.
**Remediation**: None; enforcement and verification are retained.

### P2-05: No Stress Tests — implemented (bridge stress tests added)
**Evidence**: Ignored tests `stress_10000_dispatches_keep_one_delivery_per_event` and `stress_1000_create_destroy_cycles_do_not_leak_handles` in `crates/angnative-runtime-bridge/src/lib.rs` cover 10k dispatch and 1k create/destroy cycles (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`). Dispatch assertion is one delivery per event (canonical JS thread-local path + legacy FFI fallback).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: Retained; these are Rust-side bridge stress tests, not a platform-wide stress suite.

### P2-06: No Memory Growth Analysis — implemented (bridge registry baseline)
**Evidence**: Ignored test `memory_growth_analysis_registry_returns_to_baseline` asserts the bridge handle registry returns to baseline after churn; `stress_1000_create_destroy_cycles_do_not_leak_handles` covers handle reuse (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: Retained; scope is the bridge registry, not whole-process memory profiling.

### P2-07: Duplicate Test Infrastructure — implemented (tagged custom conformance fixture)
**Evidence**: The `native-primitives-tagged-custom` conformance fixture added in this round provides the shared tagged-primitive case set used by the Rust/TypeScript sides; it does not claim full generation of one suite from the other.
**Files**: `crates/angnative-host-view/tests/conformance_spec.rs`, `packages/testing/src/conformance.spec.ts`, conformance fixture
**Remediation**: Retained fixture; complete de-duplication remains out of scope.

### P2-08: No iOS Verification
**Evidence**: iOS shell code-complete but untested.
**Files**: `platforms/ios/`, `scripts/build-ios-native.sh`
**Remediation**: Mark iOS `UNVERIFIED`. Document blocker.

### P2-09: No Desktop Verification — GPU-free integration test added
**Evidence**: `crates/angnative-desktop/tests/gpu_free_bridge_view_tree_integration.rs` verifies the GPU-free bridge/view-tree integration path (`docs/evidence/DESKTOP-GPU-FREE-INTEGRATION.md`). **Scope caveat:** real window/GPU rendering is **not verified**; no desktop GPU claim is made.
**Files**: `crates/angnative-desktop/tests/gpu_free_bridge_view_tree_integration.rs`
**Remediation**: Retained GPU-free coverage; real-GPU verification remains open.

### P2-10: CLI Build Pipeline Gaps — CLOSED
**Evidence**: `pnpm compile:aot` produces real `ngc` output in `dist/aot/*.js` and multiple ignored tests load those AOT bundles in QuickJS and assert native tree mutations (`angular_bootstrap_runs_in_quickjs`, `angular_router_bootstraps_with_outlet`, `angular_aot_i18n_translated_text_in_native_tree`, `angular_animation_engine_interpolates_native_styles`). The browser demo AOT migration is also verified: `pnpm compile:aot:web` + `pnpm bundle:aot:web` produce a bundle whose metafile has no `@angular/compiler` / `@angular/platform-browser-dynamic` input, Angular Linker processes partial declarations, and Playwright observed `Count: 0 → Count: 1` with no console/page errors (`docs/evidence/WEB-AOT-MIGRATION.md`).
**Files**: `scripts/compile-aot.mjs`, `packages/cli/src/bundle.ts`, `tsconfig.aot.web.json`, `scripts/bundle-web-aot.mjs`, `scripts/angular-linker-plugin.mjs`, `docs/UNSUPPORTED-FEATURES.md`.
**Remediation**: None for AOT output verification; the browser-demo deferral is lifted. Web size-budget gate remains a separate follow-up.

### P2-11: Size Budget Enforcement — implemented (CI wiring)
**Evidence**: The `size` job in `.github/workflows/ci.yml` installs the pinned Rust toolchain (`nightly-2026-11-14`) and runs `node scripts/check-rust-size.mjs --host` alongside the JS fixture gate. `scripts/check-rust-size.mjs` reads `rustCoreMaxBytesPerPlatform` from `packages/cli/src/budgets.json`. Apple/Windows targets remain explicit skips (`docs/evidence/RUST-SIZE-GATE.md`); no local Rust measurement is claimed. The web demo now has its own gate: `pnpm check:web-size` measured 111,977 B gzip against `webDemoGzipBytes = 128000`, with 0 forbidden packages and 0 unlinked `ɵɵngDeclare*` markers; 9 tests cover pass and all four failure codes (`docs/evidence/WEB-SIZE-BUDGET.md`). CI builds the AOT web bundle before running the gate.
**Files**: `.github/workflows/ci.yml`, `scripts/check-rust-size.mjs`, `scripts/check-web-size-budget.mjs`, `scripts/check-web-size-budget.spec.ts`, `packages/cli/src/budgets.json`, `docs/evidence/RUST-SIZE-GATE.md`, `docs/evidence/WEB-SIZE-BUDGET.md`
**Remediation**: Retained CI wiring; per-target Rust expansion requires available toolchains. Web gate is implemented; GitHub Actions execution is not claimed locally.

### P2-12: No Cross-Engine Abstraction — Gate F implemented
**Evidence**: `RuntimeBridge` is now generic over a `JsEngine` interface; `test_reference_engine_proves_bridge_engine_abstraction` proves the abstraction with the in-repo `ReferenceEngine` (`docs/evidence/RUNTIME-BRIDGE-GATE-F.md`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`, `crates/angnative-host-engine/src/reference_engine.rs`
**Remediation**: None; Gate F passed.

### P2-13: No Experimental Engine Adapters — Gate F proof, no Hermes/V8 adapter
**Evidence**: The in-repo `ReferenceEngine` (`crates/angnative-host-engine/src/reference_engine.rs`) is the proof engine for the `JsEngine` abstraction and is explicitly **not** Hermes or V8. Hermes/V8 adapters are not integrated; see `docs/evidence/RUNTIME-BRIDGE-GATE-F.md` and `docs/UNSUPPORTED-FEATURES.md`.
**Files**: `crates/angnative-host-engine/src/reference_engine.rs`, `docs/evidence/RUNTIME-BRIDGE-GATE-F.md`
**Remediation**: Gate F passed for the abstraction; no third-party engine claim is made.

### P2-14: Benchmark Infrastructure Missing — CLOSED
**Evidence**: Collector scripts and retained artifacts now exist: `scripts/measure-showcase.ps1`, `scripts/measure-frames.ps1`, `scripts/measure-multisample.ps1`, `scripts/measure-sustained.ps1`; `docs/benchmarks/frame-pacing.json`, `docs/benchmarks/emulator-multisample.json`, `docs/benchmarks/sustained/emulator-20261125-143058.json`, `docs/benchmarks/physical/results.json`, `docs/evidence/ANDROID-BENCHMARKS-MULTISAMPLE.md` and `docs/evidence/ANDROID-SUSTAINED-LOAD.md`. The historical emulator sustained run covers 10 apps with 5 cold + 5 warm starts and 3 × 30 s samples (`status=ok`, 30/30 valid, 0 errors). The physical repeat on `b6640c8f` uses the final socket-only MarketPulse APK and retains `docs/benchmarks/sustained/b6640c8f-20261126-170816.json` — 11 apps, `status=ok`, 33/33 valid, 0 errors; MarketPulse averages 1699.3 frames, 2.2% janky, p50/p95 8/22 ms, 40.34% procstat CPU (one core), warm `process_ms` mean 336.0 ms.
**Files**: `scripts/`, `docs/benchmarks/`, `docs/evidence/ANDROID-BENCHMARKS-MULTISAMPLE.md`, `docs/evidence/ANDROID-SUSTAINED-LOAD.md`, `docs/assets/market-pulse-sustained-run.log`.
**Remediation**: None. Caveats retained: one physical device, unknown qcom renderer, procstat CPU is percent of one core, top CPU is instantaneous, gfxinfo is cumulative after post-settle reset, and memory deltas include GC/settle effects.

### P2-15: Documentation Gaps
**Evidence**: Architecture docs created but not linked from code.
**Files**: `docs/architecture/`
**Remediation**: In-code references. `// See: docs/architecture/...`

### P2-16: Async JS fetch to host network — implemented (host-backed capability; socket-only market data)
**Evidence**: The QuickJS fetch shim now keeps `fetch()` promises pending until the host calls `na_onFetchResult`; host tasks are exposed through `angnative_runtime_bridge_take_host_tasks(handle)` as `fetch`, `timerSchedule` and `timerCancel`, and the host drives `na_onTimerTick`. MarketPulse uses real RxJS `interval(30000)` for bounded refresh with one request in flight; `/marketpulse/quote` resolves from the host's Coinbase WebSocket-derived cache, not a REST market endpoint. Market data itself comes only from Coinbase Exchange `matches`: 30m OHLC and integer milli-BTC volume from executed `match` price/size/time. On physical OnePlus `b6640c8f` the retained log shows empty `bars=0 state=connecting` at boot followed by `bars=1 state=live` from the first real match; failures keep last-good data with `fresh:false`. Evidence: `docs/evidence/ANDROID-MARKETPULSE.md` section 11, `docs/UNSUPPORTED-FEATURES.md`.
**Files**: `platforms/android/app/src/main/java/com/angnative/app/MarketPulseFeed.kt`, `HostCapabilities.kt`, `AngNativeView.kt`, `apps/demo-app/src/marketpulse/main.marketpulse.ts`, `crates/angnative-host-engine/src/quickjs_shims.rs`, `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: Closed for the host-backed async fetch/timer capability. SSE and REST market-data paths remain deliberately out of scope; no real-time guarantee is made.

---

## P3 - Low Issues (Polish)

### P3-01: Warning Cleanup — CLOSED
**Evidence (retained)**: No warning-cleanup follow-up is needed for the `thread_local` macro path in the supported bridge after this round; the Rust workspace check is part of the final gate artifacts in the closing commit. No local warning count is asserted here.
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None; retained evidence closes this row.

### P3-02: Unused Imports — CLOSED
**Evidence (retained)**: The previously unused `c_int` import in the runtime bridge is no longer part of the unified error/handle implementation verified by `test_metrics_error_handler_and_native_callback` and `test_thread_affinity_and_validated_handles` (`docs/evidence/RUNTIME-BRIDGE-GATE-B.md`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None; retained evidence closes this row.

### P3-03: Dead Code — CLOSED (not dead)
**Evidence**: `BatchBuilder` remains in `AngNativeView.kt` for the flagship's concept sections, which are intentionally imperative under the P0-08 flagship-only scope.
**Files**: `showcases/ecosystem-core/app/src/main/java/com/angnative/app/AngNativeView.kt:198-220`
**Remediation**: None; retained deliberately.

### P3-04: Inconsistent Error Types — implemented
**Evidence**: Unified `AngNativeError` / `ErrorCode` / `RuntimeErrorInfo` / `RuntimeMetrics` in `crates/angnative-host-engine/src/errors.rs`, surfaced via bridge accessors; verified by `test_metrics_error_handler_and_native_callback` and the Angular ErrorHandler provider tests.
**Files**: `crates/angnative-host-engine/src/errors.rs`, `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None for the unified bridge error surface.

### P3-05: Missing `Debug` Derives — implemented
**Evidence**: The new unified error/metrics types in `crates/angnative-host-engine/src/errors.rs` carry the required diagnostic derives used by the Gate B tests; no new claim is made about unrelated structs.
**Files**: `crates/angnative-host-engine/src/errors.rs`
**Remediation**: None for the types introduced this round.

### P3-06: Inconsistent Naming — implemented (bridge scope)
**Evidence**: `RuntimeController` and `RuntimeBridge` are the names used consistently in this round's Rust implementation and tests (`test_lifecycle_transitions_are_deterministic`, `test_reference_engine_proves_bridge_engine_abstraction`).
**Files**: `crates/angnative-runtime-bridge/src/lib.rs`
**Remediation**: None for bridge-scope naming; broader rename is not claimed.

### P3-07: TODO Comments — CLOSED
**Evidence (retained)**: TODOs tied to the P1/P2 closure items are resolved by the implementations and evidence in this register (unified errors, validated handles, reference engine, GPU-free desktop integration, bytecode-feasibility decision). No TODO count is asserted here.
**Files**: Multiple
**Remediation**: None; retained reasoning closes this row.

### P3-08: License Headers — status clarified
**Evidence**: No new license-header sweep was performed in this round; no file-content claim is made beyond the allowed documentation/CI scope.
**Files**: Multiple
**Remediation**: Remains a hygiene follow-up; not claimed as done or newly failing.

---

## Tracking

| Issue | Status | Owner | Target Gate |
|-------|--------|-------|-------------|
| P0-01 | ✅ Fixed | - | A |
| P0-02 | ✅ Fixed | - | A |
| P0-03 | ✅ Fixed | - | A |
| P0-04 | ✅ Fixed | - | A |
| P0-05 | ✅ Fixed | - | A |
| P0-06 | ✅ Fixed (bootstrap + click round trip verified in QuickJS) | - | C |
| P0-07 | ✅ Fixed (microtasks + polyfills + raw bundle loading) | - | C |
| P0-08 | ✅ Closed (flagship-only scope; concept sections + other shells remain imperative) | - | D |
| P1-01 | ✅ Closed (owned validated bridge handle; legacy FFI caveat retained) | - | B |
| P1-02 | ⚠️ Gate B implemented (validated handles; legacy FFI raw-pointer scope not claimed) | - | B |
| P1-03 | ⚠️ Gate B implemented (generation-counted handle registry) | - | B |
| P1-04 | ✅ Closed (unified errors; supported bridge path caveat retained) | - | B |
| P1-05 | ✅ Closed (destroy releases callbacks; supported bridge path caveat retained) | - | B |
| P1-06 | ✅ Closed (provider + QuickJS integration verified) | - | C |
| P1-07 | ✅ Closed (real AOT bundle loaded and executed) | - | C |
| P1-08 | ⬜ Deferred (self-contained IIFE bundles; no ES module loader) | - | C |
| P1-09 | ⚠️ Implemented (unified errors + Angular ErrorHandler provider tests) | - | C |
| P1-10 | ⚠️ Gate B implemented (`RuntimeController`, deterministic lifecycle) | - | B |
| P1-11 | ⚠️ Implemented (`RuntimeMetrics` accessors; no periodic-collection claim) | - | E |
| P1-12 | ✅ Implemented and device-verified (`na-candlestick`; historical 9/9 CoinGecko suite, active socket-only physical dump/pixels and post-change ABI pass) | - | D |
| P2-01 | ✅ Implemented (tagged custom + MarketPulse fixtures + host-backed async fetch/timer shims and ABI drain) | - | C |
| P2-02 | ⬜ Deferred (feasibility spike only; bytecode loading not implemented or verified) | - | C |
| P2-03 | ⚠️ Implemented (JNI RAII/failure cleanup/onDestroy tests; emulator re-run may follow) | - | D |
| P2-04 | ✅ Closed (UI-thread marshalling verified on emulator + OnePlus) | - | D |
| P2-05 | ⚠️ Implemented (bridge 10k dispatch + 1k cycle stress tests) | - | E |
| P2-06 | ⚠️ Implemented (bridge registry baseline memory-growth test) | - | E |
| P2-07 | ⚠️ Implemented (tagged custom conformance fixture; full de-dup not claimed) | - | E |
| P2-08 | 🔴 Blocked (iOS static-only; no runtime verification) | - | - |
| P2-09 | ⚠️ Implemented (GPU-free bridge/view-tree integration; real GPU not verified) | - | - |
| P2-10 | ✅ Closed (AOT output load verified in QuickJS; browser demo AOT/JIT-free verified) | - | C |
| P2-11 | ✅ Implemented (Rust CI gate wired; web bundle gate measured 111,977 B / 128,000 B gzip, 9 tests) | - | E |
| P2-12 | ⚠️ Gate F implemented (generic `RuntimeBridge` over `JsEngine`) | - | F |
| P2-13 | ⚠️ Gate F proof via in-repo `ReferenceEngine`; Hermes/V8 not integrated | - | F |
| P2-14 | ✅ Closed (emulator + physical 11-app sustained artifacts retained; 33/33 physical valid) | - | G |
| P2-15 | ✅ Done | - | A |
| P2-16 | ✅ Implemented (host-backed async fetch/timer; real RxJS interval; socket-only market data, no REST market path) | - | C |
| P3-01 | ✅ Closed (warning cleanup reconciled; no local count claimed) | - | - |
| P3-02 | ✅ Closed (unused import removed in unified bridge) | - | - |
| P3-03 | ✅ Closed (not dead — intentional concept UI) | - | D |
| P3-04 | ⚠️ Implemented (unified `AngNativeError` hierarchy) | - | - |
| P3-05 | ⚠️ Implemented for new error/metrics types | - | - |
| P3-06 | ⚠️ Implemented for bridge naming (`RuntimeBridge` / `RuntimeController`) | - | - |
| P3-07 | ✅ Closed (closure TODOs resolved by this round's implementations/evidence) | - | - |
| P3-08 | ⬜ Out of scope this round (no license-header sweep claimed) | - | - |

---

## Gate Completion Criteria

| Gate | Criteria | Status |
|------|----------|--------|
| **Gate A** | All P0 issues resolved or explicitly scoped | ✅ Passed |
| **Gate B** | Runtime lifecycle deterministic + validated handles | ✅ Passed — `docs/evidence/RUNTIME-BRIDGE-GATE-B.md` |
| **Gate C** | Angular bootstrap + execution verified **in QuickJS** (8 acceptance tests) | ✅ Passed in-process |
| **Gate D** | Android E2E round-trip verified (flagship `ecosystem-core`) | ✅ Passed with scope limitation |
| **Gate E** | All tests pass | ✅ Passed — see final gate artifacts in the closing commit |
| **Gate F** | Engine abstraction complete | ✅ Passed — `docs/evidence/RUNTIME-BRIDGE-GATE-F.md` (ReferenceEngine, not Hermes/V8) |
| **Gate G** | Benchmarks complete | ✅ Passed with scope caveats — historical emulator multi-sample/sustained retained; physical 11-app sustained repeat on `b6640c8f` is `status=ok`, 33/33 valid, final socket-only MarketPulse APK (`docs/benchmarks/sustained/b6640c8f-20261126-170816.json`) |

---

## Notes

- P0-01 through P0-05 addressed in runtime bridge refactor (thread-local, single Host, one delivery per event (canonical JS thread-local path + legacy FFI fallback))
- P0-06 and P0-07 are closed (see `docs/CODE-REVIEW-2026-11-22.md`); P0-08 is closed as a flagship-only scope decision — the flagship has a verified live Angular E2E, while other shells remain imperative concept demos by design (ADR 0003).
- 2026-11-23 reconciliation: P1-01, P1-04, P1-05, P3-01, P3-02 and P3-07 closed with retained evidence/reasoning above; Gate B and Gate F passed (see `docs/evidence/RUNTIME-BRIDGE-GATE-B.md` and `docs/evidence/RUNTIME-BRIDGE-GATE-F.md`).
- P1-08 stays deferred (self-contained IIFE bundles; no ES module loader). P2-02 is deferred after a bytecode feasibility spike (`docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md`); bytecode loading is not implemented or verified.
- iOS (P2-08) remains BLOCKED pending macOS/Xcode access; iOS runtime is static-only in this repo. Hermes/V8 are not integrated — the Gate F engine is the in-repo `ReferenceEngine` only. Physical OnePlus `b6640c8f` now has three distinct recorded results: the historical CoinGecko runtime/polling/instrumentation run (`docs/evidence/ANDROID-MARKETPULSE.md` section 9), the post-change ABI instrumentation pass (`docs/assets/market-pulse-abi-oneplus.log`), and the active socket-only match-derived bar/candle run plus physical sustained repeat (`docs/evidence/ANDROID-MARKETPULSE.md` section 11; `docs/benchmarks/sustained/b6640c8f-20261126-170816.json`). The historical nine-test MarketPulse suite is not claimed for the final socket APK; desktop real window/GPU is not verified (GPU-free integration only).
- Documentation (P2-15) completed for research phase