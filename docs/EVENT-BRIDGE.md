# Native Event Bridge — Angular `(click)` round trip

**Status (2026-11-22):** Implemented and verified in-process. `angnative-runtime-bridge`
owns Host + QuickJS + `AngNativeRuntime`, a real Angular AOT bundle boots, and a native
event reaches the Angular listener (9 ignored acceptance tests pass —
`cargo test -p angnative-runtime-bridge -- --ignored`). Loading that bundle inside a
shipping native shell on-device is the remaining integration step. See
`docs/CODE-REVIEW-2026-11-22.md` for the full evidence and honest gaps.

## What was added

### 1. Host FFI event bridge (verified on this host)
`angnative-host-ffi` now exposes:

```c
int  angnative_host_set_event_callback(AngNativeHost*, EventCallback, void* user_data);
int  angnative_host_add_event_listener(AngNativeHost*, uint64_t node_id, const char* event_name, uint64_t listener_id);
int  angnative_host_remove_event_listener(AngNativeHost*, uint64_t node_id, const char* event_name, uint64_t listener_id);
unsigned int angnative_host_dispatch_event(AngNativeHost*, uint64_t node_id, const char* event_name, const char* payload_json);
```

- Listener registry lives in `Host` (`bridge.rs`).
- `dispatch_event` invokes the installed callback once per matching listener.
- Regression test: `native_event_bridge_dispatches_to_registered_listener`
  (`cargo test -p angnative-host-ffi` → 11 passed).

### 2. QuickJS runtime API (implemented, compiled and tested)
`AngNativeRuntime` (`crates/angnative-host-engine/src/runtime.rs`) now has:

```js
na.addEventListener(nodeId, eventName, listenerId)
na.removeEventListener(nodeId, eventName, listenerId)
```
and a Rust entry point:

```rust
runtime.dispatch_event(node_id, event_name, payload_json) -> Result<u32, EngineError>
```

which calls the JS global `na_onEvent(listenerId, payloadJson)` for each
registered listener. `rustfmt --check` passes; a full `cargo test` of this
crate is blocked on Windows because `rquickjs-sys` needs elevated `patch.exe`.

## How the full round trip will work

```
native button click (Kotlin / Swift / winit)
  → angnative_host_dispatch_event(nodeId, "click", payload)
  → Host event callback (installed by the runtime bridge)
  → AngNativeRuntime::dispatch_event(...)
  → JS global na_onEvent(listenerId, payload)
  → Angular Renderer2 listener registered by @angnative/core's listen()
  → component (click) handler runs
```

## Cross-thread safety (verified on host, NOT on device)

`RuntimeBridge` records its creating `ThreadId`. Rust callers hit
`assert_owner_thread()` and panic loudly on misuse (`test_thread_affinity`).
The C ABI cannot rely on that: Android static libs are built with
`cargo build --release` and `[profile.release] panic = "abort"`, so an
`assert_owner_thread()` panic would **abort the process** instead of being
caught by `catch_unwind`. Every C ABI entry point that dereferences a bridge
handle therefore calls a non-panicking guard first:

| C ABI call | Wrong-thread result |
|---|---|
| `angnative_runtime_bridge_dispatch_event` | `DISPATCH_PANIC` (`u32::MAX` → `-1` in Kotlin via the JNI `jint` cast) |
| `angnative_runtime_bridge_load_script` | `LOAD_SCRIPT_PANIC` (`2`) |
| `angnative_runtime_bridge_get_host` | `NULL` |
| `angnative_runtime_bridge_destroy` | no-op (bridge leaked rather than torn down cross-thread) |

The guard reads the immutable `owner_thread` field through a raw pointer, so
it does not form an aliasing reference while the owner thread may be mutating
the bridge. Verified by
`test_c_abi_dispatch_from_non_owner_thread_returns_sentinel_without_panicking`:
a worker thread calls `angnative_runtime_bridge_dispatch_event` and the test
asserts both the sentinel and (via a test-only counter) that the guard was
taken rather than a panic being swallowed by `catch_unwind`.

**Not verified on device.** No `androidTest` sources, instrumentation runner,
or background-thread dispatch path exist in any shell: every Kotlin call to
`nativeRuntimeBridgeDispatchEvent` originates from an Android UI
`OnClickListener`. `EcosystemCoreActivity` loads the AOT bundle and immediately
destroys the bridge, leaving no live listener to dispatch to. No wrong-thread
JNI call has been executed on an emulator or physical device, and no Android
UI-thread marshalling is implemented in the JNI callbacks (which only
`AttachCurrentThread` for Rust→Kotlin view callbacks). Device verification
remains a concrete open task, not a claim.

## Remaining work

Done since the original note:
1. ✅ **Runtime bridge crate** — `crates/angnative-runtime-bridge` connects the Host
   event callback to `AngNativeRuntime::dispatch_event` (one owner, one event path,
   enforced thread affinity). The C ABI rejects wrong-thread calls with a sentinel
   *before* any owner-asserting method runs, so it is safe under the Android release
   profile's `panic = "abort"` (see "Cross-thread safety" above).
2. ✅ **Load the compiled Angular bundle into QuickJS** — `scripts/bundle-quickjs.mjs`
   produces a self-contained IIFE; `angular_bootstrap_runs_in_quickjs` proves the real
   AOT bundle boots and emits native `CreateNode` batches.
3. ✅ **Wire clicks** — the renderer registers listeners via `na_addEventListener` and
   `na_onEvent` routes Rust dispatch back to the Angular callback; the acceptance test
   asserts exactly one listener fires and produces a new batch.

Still open (honest):
- **On-device / shipping-shell integration.** `showcases/ecosystem-core` does load
  the real AOT bundle (`EcosystemCoreActivity.kt:170-179`) and reports
  `nodes=... (load rc=0)`, but it destroys the bridge immediately afterwards, so
  no live Angular listener survives for a tap. The other 9 showcases and
  `platforms/android` still use the legacy host + imperative batches. iOS has no
  runtime-bridge bindings. Until a shell keeps the bridge alive and a tap reaches
  Angular, the framework-level claim is "verified in the embedded runtime," not
  "runs natively." See the cross-thread section above for the separate device gap.
- **Animation-engine integration** is now verified with deterministic three-sample
  interpolation (`angular_animation_engine_interpolates_native_styles`); real-time
  easing fidelity is still unverified. DI-based `HttpClient.get()` is verified by
  `angular_http_platform_loads_in_quickjs`, template-driven `ngModel` by
  `angular_template_driven_forms_round_trip_in_quickjs`, and AOT i18n markers by
  `angular_aot_i18n_translated_text_in_native_tree` (see
  `UNSUPPORTED-FEATURES.md` and
  `docs/architecture/ARCHITECTURE-VERIFIED-2026-11-22.md`).
- The web adapter already has a complete DOM-based click path.
