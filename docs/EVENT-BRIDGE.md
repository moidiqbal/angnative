# Native Event Bridge — Angular `(click)` round trip

**Status (2026-11-21):** FFI + QuickJS-side scaffolding implemented; full
Angular-bundle-in-native runtime still pending.

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

### 2. QuickJS runtime API (source added; not compile-verified on Windows)
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

## Remaining work (not done)

1. **Runtime bridge crate** connecting `Host::set_event_callback` to
   `AngNativeRuntime::dispatch_event` (needs a crate depending on both
   `angnative-host-ffi` and `angnative-host-engine`; verify on Mac/Linux).
2. **Load the compiled Angular bundle into QuickJS** in each native shell
   (DOM shims + Angular runtime). The AOT/ngc pipeline itself is now built and
   verified (`scripts/compile-aot.mjs` → `dist/aot/` with Ivy definitions);
   what remains is wiring that compiled bundle into the native shells' QuickJS runtime.
3. **Wire platform shell clicks** to `angnative_host_dispatch_event` instead of
   the current imperative `SetTextContent` batches.

Until steps 1-3 are done, native shells still demo imperative batches; the web
adapter already has a complete DOM-based click path.
