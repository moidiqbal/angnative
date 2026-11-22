/**
 * lib.rs — Runtime bridge connecting Host event callbacks to AngNativeRuntime.
 *
 * v2 (2026-11-22 review fixes):
 * - ONE owner: RuntimeBridge owns Host + QuickJsEngine + AngNativeRuntime.
 * - ONE event path: Host.event_callback (installed here) -> js callback -> runtime.
 *   The thread-local JS_RUNTIME_DISPATCH path in host-ffi is used as the single
 *   canonical path instead; we do NOT also register event_callback, avoiding
 *   the double-dispatch bug (P0-03).
 * - Thread affinity is enforced: creating thread id is stored and asserted in
 *   every callback (P0-04). QuickJS is not Send/Sync.
 * - All extern "C" entry points are panic-protected (P1).
 */
use angnative_host_engine::{AngNativeRuntime, EngineError, JsEngine, RuntimeConfig};
use angnative_host_engine::quickjs::QuickJsEngine;
use angnative_host_ffi::bridge::{
    Host, set_js_runtime_dispatch_for_thread, clear_js_runtime_dispatch_for_thread,
};
use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::thread::{self, ThreadId};
use std::thread_local;

/// Sentinel returned by event dispatch when a panic was caught. 0 means
/// "no listener invoked"; this distinguishes a real miss from a failure.
pub const DISPATCH_PANIC: u32 = u32::MAX;
/// 0 = success, 1 = script error / invalid input, 2 = panic caught.
pub const LOAD_SCRIPT_PANIC: i32 = 2;

/// Test-only instrumentation: how many C ABI calls were rejected by the
/// owner-thread guard. In a debug (`unwind`) test build, an unguarded call
/// would still return `DISPATCH_PANIC` because `catch_unwind` swallows the
/// assertion panic — the counter lets the regression test prove the guard was
/// taken instead. Compiled out of release builds.
#[cfg(test)]
static WRONG_THREAD_FFI_REJECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

thread_local! {
    static CURRENT_BRIDGE_RUNTIME: RefCell<Option<*mut AngNativeRuntime<QuickJsEngine>>> =
        RefCell::new(None);
    /// Stable pointer to the bridge-owned Host, used by the JS-side
    /// `na_dispatchBatch` function to close the renderer -> native pipeline.
    static CURRENT_BRIDGE_HOST: RefCell<Option<*mut Host>> = RefCell::new(None);
}

/// Bridge — single authoritative owner of Host + QuickJS + Angular runtime.
pub struct RuntimeBridge {
    // Boxed so the address stays stable when the bridge itself is moved into
    // a Box by the C API. The thread-local dispatch pointer targets this
    // allocation; pointing at an inline field would dangle after a move.
    runtime: Box<AngNativeRuntime<QuickJsEngine>>,
    // Boxed for the same stability reason as `runtime`: the JS dispatch
    // function holds a thread-local raw pointer to it.
    host: Box<Host>,
    owner_thread: ThreadId,
}

impl RuntimeBridge {
    pub fn new() -> Result<Self, anyhow::Error> {
        let mut host = Box::new(Host::new());
        let engine = QuickJsEngine::new()?;
        let mut runtime = Box::new(AngNativeRuntime::new(engine, RuntimeConfig::default()));
        runtime.install_api()?;

        let owner_thread = thread::current().id();
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            *rt.borrow_mut() = Some(&mut *runtime as *mut AngNativeRuntime<QuickJsEngine>);
        });
        CURRENT_BRIDGE_HOST.with(|h| {
            *h.borrow_mut() = Some(&mut *host as *mut Host);
        });

        // THE missing pipeline closure: the Angular renderer flushes JSON
        // command batches via __angnativeDispatchBatch. Route that straight
        // into Host::process_batch_json so mutations reach the real shadow
        // tree, OpEmitter and ViewFactory (no more JS-only batches).
        runtime.engine().register_function("na_dispatchBatch", |args| {
            let Some(angnative_host_engine::JsValue::String(json)) = args.first() else {
                return Ok(angnative_host_engine::JsValue::Null);
            };
            CURRENT_BRIDGE_HOST.with(|h| {
                let Some(ptr) = *h.borrow() else {
                    return Ok(angnative_host_engine::JsValue::Null);
                };
                let host = unsafe { &mut *ptr };
                match host.process_batch_json(json.as_str()) {
                    Ok(ops) => Ok(angnative_host_engine::JsValue::String(ops)),
                    Err(e) => Err(EngineError::CallError(e)),
                }
            })
        });

        // Install the DOM/platform shims into the same engine so Angular code
        // has `window`, `document`, timers and events available. The shim sets
        // __angnativeDispatchBatch -> na_dispatchBatch.
        angnative_host_engine::quickjs_shims::install_dom_shims(runtime.engine())?;

        set_js_runtime_dispatch_for_thread(js_runtime_dispatch_callback);

        Ok(Self { runtime, host, owner_thread })
    }

    fn assert_owner_thread(&self) {
        assert_eq!(
            thread::current().id(), self.owner_thread,
            "QuickJS runtime must be used on its creating thread"
        );
    }

    pub fn dispatch_event(
        &mut self,
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> Result<u32, anyhow::Error> {
        self.assert_owner_thread();
        Ok(self.runtime.dispatch_event(node_id, event_name, payload_json)?)
    }

    pub fn runtime_mut(&mut self) -> &mut AngNativeRuntime<QuickJsEngine> {
        self.assert_owner_thread();
        &mut *self.runtime
    }

    /// Load a script (e.g. a self-contained AOT bundle) into the same engine.
    /// Raw source is evaluated as-is and pending microtasks are drained.
    pub fn load_script(&mut self, source: &str) -> Result<angnative_host_engine::JsValue, anyhow::Error> {
        self.assert_owner_thread();
        Ok(self.runtime.load_script(source)?)
    }

    pub fn host_mut(&mut self) -> &mut Host {
        &mut *self.host
    }

    pub fn host(&self) -> &Host {
        &*self.host
    }
}

impl Drop for RuntimeBridge {
    fn drop(&mut self) {
        CURRENT_BRIDGE_RUNTIME.with(|rt| *rt.borrow_mut() = None);
        CURRENT_BRIDGE_HOST.with(|h| *h.borrow_mut() = None);
        clear_js_runtime_dispatch_for_thread();
    }
}

/// Single JS dispatch callback installed on Host.
extern "C" fn js_runtime_dispatch_callback(
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
    // This callback is reached from C (Host::dispatch_event). A panic must
    // never unwind across the FFI boundary — return 0 instead.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if event_name.is_null() { return 0; }
        let event_name_str = match unsafe { CStr::from_ptr(event_name) }.to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        let payload_str = if payload_json.is_null() {
            "{}"
        } else {
            match unsafe { CStr::from_ptr(payload_json) }.to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            let borrow = rt.borrow();
            match borrow.as_ref() {
                Some(ptr) => {
                    let runtime = unsafe { &mut **ptr };
                    match runtime.dispatch_event(node_id, event_name_str, payload_str) {
                        Ok(invoked) => invoked,
                        Err(_) => 0,
                    }
                }
                None => 0,
            }
        })
    }));
    result.unwrap_or(DISPATCH_PANIC)
}

#[repr(C)]
pub struct AngNativeRuntimeBridge {
    _private: [u8; 0],
}

/// Read the immutable owner thread id from a C ABI handle without forming a
/// reference to the whole bridge. The caller may be on the wrong thread while
/// the owner thread is mutating other fields, so creating an aliasing
/// `&RuntimeBridge` / `&mut RuntimeBridge` here would be unsound. `owner_thread`
/// is never written after construction, so this raw field read is safe.
unsafe fn bridge_owner_thread(handle: *mut AngNativeRuntimeBridge) -> Option<ThreadId> {
    if handle.is_null() {
        return None;
    }
    Some(std::ptr::addr_of!((*(handle as *mut RuntimeBridge)).owner_thread).read())
}

/// Non-panicking guard shared by every C ABI entry point that dereferences a
/// bridge handle. On the wrong thread these functions must return a sentinel
/// *before* touching QuickJS (which is not Send/Sync) or tripping an
/// `assert_owner_thread()` that would abort an Android release build.
fn bridge_is_owner_thread(handle: *mut AngNativeRuntimeBridge) -> bool {
    let owner = unsafe { bridge_owner_thread(handle) };
    let is_owner = matches!(owner, Some(id) if thread::current().id() == id);
    #[cfg(test)]
    if owner.is_some() && !is_owner {
        WRONG_THREAD_FFI_REJECTIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    is_owner
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_create() -> *mut AngNativeRuntimeBridge {
    let result = std::panic::catch_unwind(|| {
        match RuntimeBridge::new() {
            Ok(b) => Box::into_raw(Box::new(b)) as *mut AngNativeRuntimeBridge,
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_destroy(handle: *mut AngNativeRuntimeBridge) {
    if !bridge_is_owner_thread(handle) { return; }
    let _ = std::panic::catch_unwind(|| {
        drop(Box::from_raw(handle as *mut RuntimeBridge));
    });
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_dispatch_event(
    handle: *mut AngNativeRuntimeBridge,
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
    if handle.is_null() || event_name.is_null() { return 0; }
    // Guard BEFORE any owner-asserting call or QuickJS access. Under
    // `panic = "abort"` (Android release), `catch_unwind` cannot save us from
    // `assert_owner_thread()`, so a wrong-thread call must return the sentinel
    // without unwinding.
    if !bridge_is_owner_thread(handle) { return DISPATCH_PANIC; }
    let name = match CStr::from_ptr(event_name).to_str() { Ok(s) => s, Err(_) => return 0 };
    let payload = if payload_json.is_null() {
        "{}"
    } else {
        match CStr::from_ptr(payload_json).to_str() { Ok(s) => s, Err(_) => return 0 }
    };
    let bridge = &mut *(handle as *mut RuntimeBridge);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bridge.dispatch_event(node_id, name, payload).unwrap_or(0)
    }));
    result.unwrap_or(DISPATCH_PANIC)
}

/// Load a raw JS source string into the runtime. Returns 0 on success,
/// 1 on invalid arguments or script error. No string interpolation is used.
#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_load_script(
    handle: *mut AngNativeRuntimeBridge,
    source: *const c_char,
) -> i32 {
    if handle.is_null() || source.is_null() {
        return 1;
    }
    if !bridge_is_owner_thread(handle) {
        return LOAD_SCRIPT_PANIC;
    }
    let source = match CStr::from_ptr(source).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let bridge = &mut *(handle as *mut RuntimeBridge);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bridge.load_script(source).is_ok()
    }));
    match result {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => LOAD_SCRIPT_PANIC,
    }
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_get_host(
    handle: *mut AngNativeRuntimeBridge,
) -> *mut Host {
    if !bridge_is_owner_thread(handle) { return std::ptr::null_mut(); }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let bridge = &mut *(handle as *mut RuntimeBridge);
        bridge.host_mut() as *mut Host
    }));
    result.unwrap_or(std::ptr::null_mut())
}

pub fn create_runtime_bridge() -> Result<RuntimeBridge, anyhow::Error> {
    RuntimeBridge::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use angnative_host_engine::JsValue;

    #[test]
    fn test_create_runtime_bridge() {
        assert!(create_runtime_bridge().is_ok());
    }

    #[test]
    fn test_dispatch_event_returns_zero_when_no_listeners() {
        let mut bridge = create_runtime_bridge().unwrap();
        assert_eq!(bridge.dispatch_event(1, "press", "{}").unwrap(), 0);
    }

    /// End-to-end proof of the single event pipeline (review P0-03):
    /// a JS-registered listener must fire EXACTLY ONCE through both the direct
    /// bridge path and the Host/thread-local path — never twice, never zero.
    #[test]
    fn test_event_dispatches_exactly_once_end_to_end() {
        use angnative_host_engine::JsValue;

        let mut bridge = create_runtime_bridge().unwrap();
        bridge.runtime_mut().eval(r#"
            var _e2eCount = 0;
            var _e2eIds = [];
            function na_onEvent(listenerId, payloadJson) {
                _e2eCount++;
                _e2eIds.push(listenerId);
            }
            na_addEventListener(42, 'press', 7);
        "#).unwrap();

        // Path A: direct bridge dispatch (the Android JNI path).
        assert_eq!(bridge.dispatch_event(42, "press", "{\"a\":1}").unwrap(), 1);
        assert_eq!(
            bridge.runtime_mut().eval("_e2eCount").unwrap(),
            JsValue::Number(1.0)
        );

        // Path B: Host::dispatch_event -> thread-local JS callback (legacy
        // shell path). Must invoke the listener once, not double-fire through
        // both the JS callback and the legacy FFI callback.
        assert_eq!(bridge.host_mut().dispatch_event(42, "press", "{\"a\":2}"), 1);
        assert_eq!(
            bridge.runtime_mut().eval("_e2eCount").unwrap(),
            JsValue::Number(2.0)
        );
        assert_eq!(
            bridge.runtime_mut().eval("_e2eIds.join(',')").unwrap(),
            JsValue::String("7,7".to_string())
        );

        // Wrong node / event must not fire anything.
        assert_eq!(bridge.dispatch_event(99, "press", "{}").unwrap(), 0);
        assert_eq!(bridge.dispatch_event(42, "release", "{}").unwrap(), 0);
        assert_eq!(
            bridge.runtime_mut().eval("_e2eCount").unwrap(),
            JsValue::Number(2.0)
        );
    }

    /// A script can be loaded raw (no interpolation) and its microtasks are
    /// drained by the runtime — the foundation for loading a self-contained
    /// AOT bundle into QuickJS.
    #[test]
    fn test_load_script_and_drain_microtasks() {
        use angnative_host_engine::JsValue;

        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(r#"
            var _loaded = 0;
            var _tick = 0;
            queueMicrotask(function () { _loaded = 42; _tick++; });
        "#).unwrap();

        assert_eq!(bridge.runtime_mut().eval("_loaded").unwrap(), JsValue::Number(42.0));
        assert_eq!(bridge.runtime_mut().eval("_tick").unwrap(), JsValue::Number(1.0));
    }

    /// Acceptance test for Angular bootstrap in QuickJS (P0-06).
    ///
    /// Requires the generated bundle:
    ///   pnpm compile:aot && pnpm bundle:quickjs
    /// Ignored by default because `dist/` is gitignored and not available in a
    /// fresh checkout. Run explicitly with:
    ///   cargo test -p angnative-runtime-bridge -- --ignored
    #[test]
    #[ignore = "requires generated dist/aot/quickjs-bundle.js"]
    fn angular_bootstrap_runs_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/quickjs-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot && pnpm bundle:quickjs` first");

        let mut bridge = create_runtime_bridge().unwrap();
        // Capture any rejected bootstrap promise so the test can assert on it.
        let wrapped = r#"
            var _bootstrapErr = null;
            var _origError = console.error;
            console.error = function () {
                var e = arguments[1];
                _bootstrapErr = String(e && e.message);
                _origError.apply(console, arguments);
            };
            try {
            "# .to_string()
            + &source
            + r#"
            } catch (e) {
                _bootstrapErr = 'THROWN: ' + String(e && e.message);
            }
            "#;

        bridge.load_script(&wrapped).expect("bundle must parse and run");

        assert_eq!(
            bridge.runtime_mut().eval("_bootstrapErr").unwrap(),
            JsValue::Null,
            "Angular bootstrap rejected"
        );
        assert_eq!(
            bridge.runtime_mut().eval("typeof __angnativeBatches").unwrap(),
            JsValue::String("object".to_string())
        );
        let batches = match bridge.runtime_mut().eval("__angnativeBatches.length").unwrap() {
            JsValue::Number(n) => n as u32,
            other => panic!("unexpected batch count: {other:?}"),
        };
        assert!(batches > 0, "renderer must emit at least one command batch");
        let json = bridge
            .runtime_mut()
            .eval("JSON.stringify(__angnativeBatches)")
            .unwrap();
        match json {
            JsValue::String(s) => assert!(
                s.contains("CreateNode"),
                "native tree must be built; got {s}"
            ),
            other => panic!("unexpected batch JSON: {other:?}"),
        }

        // THE REAL PROOF: batches must have flowed through the native hook
        // into the Host's own MutationEngine / OpEmitter — not just into a
        // JS array. This closes the renderer -> native pipeline.
        assert_eq!(
            bridge.runtime_mut().eval("typeof __angnativeDispatchBatch").unwrap(),
            JsValue::String("function".to_string()),
            "native batch dispatch hook must be installed"
        );
        let host_nodes = bridge.host_mut().engine().size();
        assert!(host_nodes > 0, "Host shadow tree must contain native nodes");
        let last_ops = bridge.runtime_mut().eval("String(__angnativeLastOps)").unwrap();
        match last_ops {
            JsValue::String(s) => assert!(
                s.contains("Create") || s.contains("SetText") || s.contains("SetProps"),
                "Host must have emitted native ops; got {s}"
            ),
            other => panic!("unexpected native ops: {other:?}"),
        }

        // Full round trip: locate the Angular-registered listener from the
        // emitted batch, dispatch a native event through the RuntimeBridge,
        // and assert the component handler produced new native mutations.
        let listener = bridge
            .runtime_mut()
            .eval(
                r#"(function () {
                    for (var i = 0; i < __angnativeBatches.length; i++) {
                        var b = JSON.parse(__angnativeBatches[i]);
                        for (var j = 0; j < b.commands.length; j++) {
                            var c = b.commands[j];
                            if (c.key && c.key.indexOf('listener:') === 0) {
                                var parts = c.key.split(':');
                                return c.id + '|' + parts[1] + '|' + parts[2];
                            }
                        }
                    }
                    return null;
                })()"#,
            )
            .unwrap();
        let JsValue::String(listener) = listener else {
            panic!("no Angular listener registration found in the emitted batches");
        };
        let parts: Vec<&str> = listener.split('|').collect();
        assert_eq!(parts.len(), 3, "bad listener descriptor: {listener}");
        let node_id: u64 = parts[0].parse().expect("node id");
        let event_name = parts[1];
        let _listener_id: u64 = parts[2].parse().expect("listener id");

        let before = batches;
        let invoked = bridge
            .dispatch_event(node_id, event_name, "{\"source\":\"native\"}")
            .expect("native event dispatch must succeed");
        assert_eq!(invoked, 1, "exactly one Angular listener must fire");
        let after = match bridge.runtime_mut().eval("__angnativeBatches.length").unwrap() {
            JsValue::Number(n) => n as u32,
            other => panic!("unexpected batch count: {other:?}"),
        };
        assert!(
            after > before,
            "Angular click handler must emit new native mutations"
        );
        let ops_after_click = bridge.runtime_mut().eval("String(__angnativeLastOps)").unwrap();
        match ops_after_click {
            JsValue::String(s) => assert!(
                s.contains("SetText") || s.contains("SetProps") || s.contains("Create"),
                "click must produce native ops, not just JS JSON; got {s}"
            ),
            other => panic!("unexpected native ops: {other:?}"),
        }
    }

    /// Angular Forms compatibility (Phase 1). Requires:
    ///   pnpm bundle:forms
    #[test]
    #[ignore = "requires generated dist/aot/forms-bundle.js"]
    fn angular_forms_work_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/forms-bundle.js"
        );
        let source = std::fs::read_to_string(path).expect("run `pnpm bundle:forms` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("forms bundle must load");

        let result = bridge
            .load_script(
                r#"
                var f = new __ngForms.FormControl('', __ngForms.Validators.required);
                var initial = f.value + '|' + f.invalid;
                f.setValue('hello');
                var updated = f.value + '|' + f.valid;
                var g = new __ngForms.FormGroup({
                    name: new __ngForms.FormControl('', __ngForms.Validators.required)
                });
                var groupInvalid = g.invalid;
                g.get('name').setValue('AngNative');
                var groupValid = g.valid;
                [initial, updated, groupInvalid, groupValid].join(',')
            "#,
            )
            .expect("forms API must run in QuickJS");
        assert_eq!(
            result,
            JsValue::String("|true,hello|true,true,true".to_string())
        );
    }

    /// Template-driven Forms compatibility. Requires:
    ///   pnpm compile:aot:forms && pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/forms-app-bundle.js"]
    fn angular_template_driven_forms_round_trip_in_quickjs() {
        let handle = std::thread::Builder::new()
            .name("forms-test-thread".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_angular_template_driven_forms_round_trip_in_quickjs)
            .unwrap();
        handle.join().unwrap();
    }

    fn run_angular_template_driven_forms_round_trip_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/forms-app-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot:forms && pnpm bundle:framework` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("forms AOT bundle must load");

        assert_eq!(
            bridge
                .runtime_mut()
                .eval("typeof __formsError === 'undefined' ? null : __formsError")
                .unwrap(),
            JsValue::Null,
            "Forms AOT bootstrap rejected"
        );
        assert_eq!(
            bridge.runtime_mut().eval("__formsModel()").unwrap(),
            JsValue::String("initial".to_string())
        );

        // Locate the DefaultValueAccessor input listener emitted by Angular.
        let listener = bridge
            .runtime_mut()
            .eval(
                r#"(function () {
                    for (var i = 0; i < __angnativeBatches.length; i++) {
                        var b = JSON.parse(__angnativeBatches[i]);
                        for (var j = 0; j < b.commands.length; j++) {
                            var c = b.commands[j];
                            if (c.key && c.key.indexOf('listener:input:') === 0) {
                                var parts = c.key.split(':');
                                return c.id + '|' + parts[1] + '|' + parts[2];
                            }
                        }
                    }
                    return null;
                })()"#,
            )
            .unwrap();
        let JsValue::String(listener) = listener else {
            panic!("no ngModel input listener registration found in the emitted batches");
        };
        let parts: Vec<&str> = listener.split('|').collect();
        assert_eq!(parts.len(), 3, "bad listener descriptor: {listener}");
        let input_node_id: u64 = parts[0].parse().expect("node id");
        assert_eq!(parts[1], "input");

        // JS model -> native tree.
        bridge
            .load_script("__setFormsModel('set-from-js'); 0;")
            .expect("model setter must run");
        assert_eq!(
            bridge.runtime_mut().eval("__formsModel()").unwrap(),
            JsValue::String("set-from-js".to_string())
        );
        let snapshot = bridge.host_mut().engine().snapshot();
        let input_node = snapshot
            .nodes
            .iter()
            .find(|node| node.id == input_node_id)
            .expect("native input node missing");
        assert_eq!(
            input_node.tag, "na-text-input",
            "<input> must map to the native text input primitive"
        );
        assert_eq!(
            input_node.properties.get("value").and_then(|v| v.as_str()),
            Some("set-from-js"),
            "native input value must reflect the JS model"
        );
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.text_content.as_deref() == Some("set-from-js")),
            "interpolated native text must reflect the JS model"
        );

        // Native input event -> Angular ngModel -> component model.
        let invoked = bridge
            .dispatch_event(
                input_node_id,
                "input",
                r#"{"target":{"value":"typed-from-native"}}"#,
            )
            .expect("native input event dispatch must succeed");
        assert_eq!(invoked, 1, "exactly one ngModel input listener must fire");
        bridge.load_script("__flushForms(); 0;").unwrap();
        assert_eq!(
            bridge.runtime_mut().eval("__formsModel()").unwrap(),
            JsValue::String("typed-from-native".to_string()),
            "native input event must update the ngModel-backed component model"
        );
        // Angular does not write the CVA value back for a user-originated
        // input event; the native widget already holds the typed value. The
        // observable model and its interpolated native label must still update.
        let snapshot = bridge.host_mut().engine().snapshot();
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.text_content.as_deref() == Some("typed-from-native")),
            "interpolated native text must reflect the event-driven model update"
        );
    }

    /// AOT i18n markers in a real template. Requires:
    ///   pnpm compile:aot:i18n && pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/i18n-app-bundle.js"]
    fn angular_aot_i18n_translated_text_in_native_tree() {
        let handle = std::thread::Builder::new()
            .name("i18n-test-thread".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_angular_aot_i18n_translated_text_in_native_tree)
            .unwrap();
        handle.join().unwrap();
    }

    fn run_angular_aot_i18n_translated_text_in_native_tree() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/i18n-app-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot:i18n && pnpm bundle:framework` first");
        assert!(
            source.contains("$localize"),
            "AOT bundle must contain the $localize marker emitted by ngc"
        );
        assert!(
            source.contains("i18nExp"),
            "AOT bundle must contain Angular's i18n template instruction"
        );

        let mut bridge = create_runtime_bridge().unwrap();
        // Register before evaluating the bundle: ngc initializes the i18n
        // message constant while defining the component.
        bridge
            .load_script(
                r#"
                $localize.loadTranslations({
                    ':\u241F371b83d16c77b75bbf80d741d66e73c5a7b9eaea\u241F1363615954628417873:Hello \uFFFD0\uFFFD:INTERPOLATION:!': 'Assalam-o-Alaikum \uFFFD0\uFFFD!'
                });
                0;
            "#,
            )
            .expect("translation registration must run");

        bridge.load_script(&source).expect("i18n AOT bundle must load");
        assert_eq!(
            bridge
                .runtime_mut()
                .eval("typeof __i18nError === 'undefined' ? null : __i18nError")
                .unwrap(),
            JsValue::Null,
            "i18n AOT bootstrap rejected"
        );

        let snapshot = bridge.host_mut().engine().snapshot();
        assert!(
            snapshot.nodes.iter().any(|node| {
                node.text_content.as_deref() == Some("Assalam-o-Alaikum AngNative!")
            }),
            "native tree must contain the translated, interpolated text node; got {:?}",
            snapshot.nodes.iter().map(|n| n.text_content.clone()).collect::<Vec<_>>()
        );
    }

    /// Real Angular animation engine over the native renderer. Requires:
    ///   pnpm compile:aot:animations && pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/animations-app-bundle.js"]
    fn angular_animation_engine_interpolates_native_styles() {
        let handle = std::thread::Builder::new()
            .name("animations-test-thread".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_angular_animation_engine_interpolates_native_styles)
            .unwrap();
        handle.join().unwrap();
    }

    fn run_angular_animation_engine_interpolates_native_styles() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/animations-app-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot:animations && pnpm bundle:framework` first");
        assert!(
            source.contains("fade"),
            "AOT bundle must contain the real Angular animation trigger"
        );

        let mut bridge = create_runtime_bridge().unwrap();
        bridge
            .load_script(&source)
            .expect("animations AOT bundle must load");
        assert_eq!(
            bridge
                .runtime_mut()
                .eval("typeof __animationsError === 'undefined' ? null : __animationsError")
                .unwrap(),
            JsValue::Null,
            "animations AOT bootstrap rejected"
        );

        // Drive the real Angular trigger. The driver applies start, midpoint
        // and end samples synchronously; the factory flushes them as one batch.
        bridge
            .load_script("__resetAnimationEvents(); __setAnimationState('visible'); 0;")
            .expect("state transition must run");
        bridge.runtime_mut().drain_microtasks().unwrap();

        let style_values = bridge
            .runtime_mut()
            .eval(
                r#"(function () {
                    var values = [];
                    for (var i = 0; i < __angnativeBatches.length; i++) {
                        var batch = JSON.parse(__angnativeBatches[i]);
                        for (var j = 0; j < batch.commands.length; j++) {
                            var command = batch.commands[j];
                            if (command.kind === 'SetStyle' && command.key === 'opacity') {
                                values.push(command.value);
                            }
                        }
                    }
                    return values.join(',');
                })()"#,
            )
            .unwrap();
        let style_values = match style_values {
            JsValue::String(value) => value,
            other => panic!("expected opacity style history, got {:?}", other),
        };
        let start = style_values
            .find("0.5")
            .unwrap_or_else(|| panic!("no intermediate opacity mutation; history={}", style_values));
        assert!(
            style_values[..start].contains('0'),
            "start opacity must precede the midpoint: {}",
            style_values
        );
        assert!(
            style_values[start..].contains('1'),
            "final opacity must follow the midpoint: {}",
            style_values
        );

        let events = bridge
            .runtime_mut()
            .eval("__animationEvents ? __animationEvents().join(',') : ''")
            .unwrap();
        assert_eq!(
            events,
            JsValue::String("start,done".to_string()),
            "Angular trigger callbacks must fire; got {:?}",
            events
        );

        let snapshot = bridge.host_mut().engine().snapshot();
        assert!(
            snapshot.nodes.iter().any(|node| {
                node.styles.get("opacity").map(String::as_str) == Some("1")
            }),
            "native tree must end at opacity 1; got {:?}",
            snapshot.nodes.iter().map(|n| n.styles.clone()).collect::<Vec<_>>()
        );
    }

    /// Router API compatibility. Requires: pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/router-bundle.js"]
    fn angular_router_loads_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/router-bundle.js"
        );
        let source = std::fs::read_to_string(path).expect("run `pnpm bundle:framework` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("router bundle must load");
        let result = bridge
            .load_script(
                r#"[typeof __ngRouter.provideRouter,
                    typeof __ngRouter.RouterModule,
                    typeof __ngRouter.Router,
                    typeof __ngRouter.ROUTES].join(',')"#,
            )
            .unwrap();
        assert_eq!(
            result,
            JsValue::String("function,function,function,object".to_string())
        );
        // The history/location shims are the platform surface the Router uses.
        bridge
            .load_script("history.pushState(null, '', '/products/42');")
            .unwrap();
        assert_eq!(
            bridge.runtime_mut().eval("location.pathname").unwrap(),
            JsValue::String("/products/42".to_string())
        );
    }

    /// Animations API compatibility. Requires: pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/animations-bundle.js"]
    fn angular_animations_load_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/animations-bundle.js"
        );
        let source = std::fs::read_to_string(path).expect("run `pnpm bundle:framework` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("animations bundle must load");
        let result = bridge
            .load_script(
                r#"[typeof __ngAnimations,
                    typeof __ngAnimations.trigger,
                    typeof __ngAnimations.style,
                    typeof __ngAnimations.animate].join(',')"#,
            )
            .unwrap();
        assert_eq!(
            result,
            JsValue::String("object,function,function,function".to_string())
        );
    }

    /// HttpClient platform compatibility. Requires: pnpm bundle:framework
    ///
    /// Verifies the real @angular/common/http bundle loads, that the
    /// host-wirable fetch surface returns a proper Response, and that
    /// DI-based `HttpClient.get()` resolves and delivers the native body.
    ///
    /// Root cause of the former NG0201 (verified against Angular 22 source):
    /// `createEnvironmentInjector()` builds an R3Injector whose scopes are
    /// `{'environment'}`. Its parent was a plain `Injector.create()`, which has
    /// no `root` scope, so every `providedIn: 'root'` token was unresolvable.
    /// The first missing token was `HTTP_FETCH_MAX_RESPONSE_SIZE` (injected by
    /// FetchBackend), followed by `PendingTasks` and
    /// `REQUESTS_CONTRIBUTE_TO_STABILITY`. Angular's own
    /// `internalCreateApplication` creates the root injector with
    /// `{provide: INJECTOR_SCOPE, useValue: 'root'}`; doing the same here makes
    /// all root-scoped providers resolve from the same pnpm module instance.
    #[test]
    #[ignore = "requires generated dist/aot/http-bundle.js"]
    fn angular_http_platform_loads_in_quickjs() {
        // Angular's DI/HttpClient graph is large; run on a dedicated thread
        // with a 32 MB stack (same pattern as the router acceptance test).
        let handle = std::thread::Builder::new()
            .name("http-test-thread".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_angular_http_platform_loads_in_quickjs)
            .unwrap();
        handle.join().unwrap();
    }

    fn run_angular_http_platform_loads_in_quickjs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/http-bundle.js"
        );
        let source = std::fs::read_to_string(path).expect("run `pnpm bundle:framework` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("http bundle must load");

        assert_eq!(
            bridge
                .load_script("[typeof __ngHttp.HttpClient, typeof __ngHttp.provideHttpClient].join(',')")
                .unwrap(),
            JsValue::String("function,function".to_string())
        );

        bridge
            .load_script(
                r#"
                __angnativeRegisterFetchHandler(function (req) {
                    return {
                        status: 200,
                        body: { message: 'hello-from-native', method: req.method }
                    };
                });
                var _httpResponse = null;
                fetch('/api/test').then(function (r) {
                    return r.json().then(function (j) {
                        _httpResponse = r.status + '|' + j.message + '|' + j.method;
                    });
                });
            "#,
            )
            .expect("fetch shim must run");
        assert_eq!(
            bridge.runtime_mut().eval("_httpResponse").unwrap(),
            JsValue::String("200|hello-from-native|GET".to_string())
        );

        // DI-based HttpClient.get(): same fetch handler, but through the real
        // Angular HttpHandler/FetchBackend pipeline. `HttpClient.get()` defaults
        // to observe:'body', so `next` receives the decoded native body.
        bridge
            .load_script(
                r#"
                var _httpDiBody = null;
                var _httpDiStatus = null;
                var _httpDiError = null;
                try {
                    var _httpParent = __ngHttp.Injector.create({ providers: [] });
                    var _httpEnv = __ngHttp.createEnvironmentInjector([
                        __ngHttp.provideZonelessChangeDetection(),
                        __ngHttp.provideHttpClient(),
                        { provide: __ngHttp.INJECTOR_SCOPE, useValue: 'root' }
                    ], _httpParent);
                    var _httpClient = _httpEnv.get(__ngHttp.HttpClient);
                    _httpClient.get('/api/x').subscribe({
                        next: function (body) {
                            _httpDiBody = (body && body.message) + '|' +
                                (body && body.method);
                        },
                        error: function (err) {
                            _httpDiError = String(err && err.message || err);
                        }
                    });
                    _httpClient.get('/api/x', { observe: 'response' }).subscribe({
                        next: function (res) {
                            if (res && res.type === 4) _httpDiStatus = res.status;
                        },
                        error: function (err) {
                            _httpDiError = String(err && err.message || err);
                        }
                    });
                } catch (e) {
                    _httpDiError = String(e && e.message || e);
                }
                0; // return a primitive: subscribe() returns a cyclic Subscription
            "#,
            )
            .expect("HttpClient.get must be dispatchable");

        assert_eq!(
            bridge.runtime_mut().eval("_httpDiError").unwrap(),
            JsValue::Null,
            "DI-based HttpClient.get must not error"
        );
        assert_eq!(
            bridge.runtime_mut().eval("_httpDiBody").unwrap(),
            JsValue::String("hello-from-native|GET".to_string()),
            "HttpClient.get must deliver the host fetch handler body"
        );
        assert_eq!(
            bridge.runtime_mut().eval("_httpDiStatus").unwrap(),
            JsValue::Number(200.0),
            "HttpResponse status must come from the native fetch handler"
        );
    }

    /// Full component-based Router navigation in QuickJS.
    /// Requires: AOT compile + pnpm bundle:framework
    #[test]
    #[ignore = "requires generated dist/aot/router-app-bundle.js"]
    fn angular_router_bootstraps_with_outlet() {
        let handle = std::thread::Builder::new()
            .name("router-test-thread".into())
            .stack_size(32 * 1024 * 1024) // 32 MB stack
            .spawn(|| {
                run_angular_router_bootstraps_with_outlet();
            })
            .unwrap();
        handle.join().unwrap();
    }

    fn run_angular_router_bootstraps_with_outlet() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/router-app-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run ngc with tsconfig.aot.router.json && pnpm bundle:framework first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("router app bundle must load");
        bridge.runtime_mut().drain_microtasks().unwrap();

        assert_eq!(
            bridge.runtime_mut().eval("typeof __routerError === 'undefined' ? 'none' : __routerError").unwrap(),
            JsValue::String("none".to_string()),
            "bootstrap must not reject"
        );
        assert_eq!(
            bridge.runtime_mut().eval("typeof __navigate").unwrap(),
            JsValue::String("function".to_string())
        );

        // The root component, <router-outlet> anchor, and initial route (HOME ROUTE)
        // must be built in the native tree.
        let initial_batches = bridge
            .runtime_mut()
            .eval("JSON.stringify(__angnativeBatches)")
            .unwrap();
        match initial_batches {
            JsValue::String(s) => {
                assert!(
                    s.contains("CreateNode") && s.contains("InsertChild"),
                    "router outlet must produce native nodes; got {s}"
                );
                assert!(
                    s.contains("HOME ROUTE"),
                    "initial navigation must render HOME ROUTE; got {s}"
                );
            }
            other => panic!("unexpected batch JSON: {other:?}"),
        }

        // Perform imperative route navigation to /about.
        bridge
            .load_script(r#"
                try {
                    globalThis.__navDone = false;
                    __navigate('/about').then(
                        function(val) { globalThis.__navDone = true; globalThis.__navResult = 'ok:' + val; },
                        function(err) { globalThis.__navDone = true; globalThis.__navResult = 'err:' + (err && err.message || err); }
                    );
                } catch (e) {
                    globalThis.__navThrow = String(e && e.message || e);
                }
            "#)
            .expect("navigation must be dispatchable");
        bridge.runtime_mut().drain_microtasks().unwrap();

        assert_eq!(
            bridge
                .runtime_mut()
                .eval("typeof __navThrow === 'undefined' ? 'none' : __navThrow")
                .unwrap(),
            JsValue::String("none".to_string()),
            "navigateByUrl must not throw synchronously"
        );
        assert_eq!(
            bridge.runtime_mut().eval("globalThis.__navResult").unwrap(),
            JsValue::String("ok:true".to_string()),
            "navigation to /about must succeed"
        );
        assert_eq!(
            bridge.runtime_mut().eval("location.pathname").unwrap(),
            JsValue::String("/about".to_string()),
            "location pathname must be /about"
        );
        assert_eq!(
            bridge.runtime_mut().eval("globalThis.__router.url").unwrap(),
            JsValue::String("/about".to_string()),
            "router URL must be /about"
        );

        // Verify that ABOUT ROUTE component was rendered in the native tree.
        let final_batches = bridge
            .runtime_mut()
            .eval("JSON.stringify(__angnativeBatches)")
            .unwrap();
        match final_batches {
            JsValue::String(s) => {
                assert!(
                    s.contains("ABOUT ROUTE"),
                    "navigation to /about must render ABOUT ROUTE into native tree; got {s}"
                );
            }
            other => panic!("unexpected final batch JSON: {other:?}"),
        }
    }

    #[test]
    fn test_thread_affinity() {
        // Creating on this thread, using on another must panic (not UB).
        let bridge = create_runtime_bridge().unwrap();
        let handle = Box::into_raw(Box::new(bridge)) as *mut AngNativeRuntimeBridge;
        let h = handle as usize;
        let joined = thread::spawn(move || {
            let ptr = h as *mut AngNativeRuntimeBridge;
            let b = unsafe { &mut *(ptr as *mut RuntimeBridge) };
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = b.dispatch_event(1, "press", "{}");
            })).is_err()
        }).join();
        assert!(joined.unwrap_or(false), "cross-thread use must panic");
        unsafe { angnative_runtime_bridge_destroy(handle) };
    }

    /// Regression for the Android release build: `panic = "abort"` means the
    /// `catch_unwind` inside the C ABI cannot catch `assert_owner_thread()`, so
    /// a wrong-thread JNI dispatch would kill the app. The C ABI must reject
    /// the call with a sentinel *before* any owner-asserting method runs.
    #[test]
    fn test_c_abi_dispatch_from_non_owner_thread_returns_sentinel_without_panicking() {
        use std::sync::atomic::Ordering;
        use std::ffi::CString;

        let handle = angnative_runtime_bridge_create();
        assert!(!handle.is_null(), "bridge creation must succeed");

        let before = WRONG_THREAD_FFI_REJECTIONS.load(Ordering::SeqCst);
        // Raw pointers are not Send; carry the address as usize so no unsafe
        // Send impl is needed to move it into the worker thread.
        let address = handle as usize;
        let result = thread::spawn(move || {
            let foreign = address as *mut AngNativeRuntimeBridge;
            let name = CString::new("press").unwrap();
            let payload = CString::new("{}").unwrap();
            unsafe {
                angnative_runtime_bridge_dispatch_event(
                    foreign,
                    1,
                    name.as_ptr(),
                    payload.as_ptr(),
                )
            }
        })
        .join()
        .expect("wrong-thread C ABI dispatch must not panic");

        assert_eq!(
            result,
            DISPATCH_PANIC,
            "wrong-thread C ABI dispatch must return the sentinel"
        );
        assert_eq!(
            WRONG_THREAD_FFI_REJECTIONS.load(Ordering::SeqCst),
            before + 1,
            "the C ABI guard must reject the call, not rely on a caught panic \
             (which would abort under the Android release profile)"
        );

        unsafe { angnative_runtime_bridge_destroy(handle) };
    }

    /// Third-party spot-check: RxJS 7.8.2 is a real, DOM-free npm package
    /// already pinned in this repo's lockfile. Loading its unmodified bundle
    /// into the embedded runtime proves a non-Angular third-party library can
    /// execute on the native path. Requires: `pnpm bundle:third-party`.
    #[test]
    #[ignore = "requires generated dist/aot/rxjs-bundle.js"]
    fn third_party_rxjs_runs_unchanged_in_quickjs() {
        let handle = std::thread::Builder::new()
            .name("rxjs-spotcheck-thread".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_third_party_rxjs_spotcheck)
            .unwrap();
        handle.join().unwrap();
    }

    fn run_third_party_rxjs_spotcheck() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/rxjs-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm bundle:third-party` first");
        let mut bridge = create_runtime_bridge().unwrap();
        bridge
            .load_script(&source)
            .expect("unmodified RxJS bundle must load in QuickJS");

        bridge
            .load_script(
                r#"
                globalThis.__rxjsValues = [];
                globalThis.__rxjsComplete = false;
                globalThis.__rxjsError = null;
                try {
                  __thirdPartyRxjs.of(1, 2, 3).pipe(
                    __thirdPartyRxjsOperators.map(function (x) { return x * 2; })
                  ).subscribe({
                    next: function (v) { __rxjsValues.push(v); },
                    error: function (e) { globalThis.__rxjsError = String(e); },
                    complete: function () { globalThis.__rxjsComplete = true; }
                  });
                } catch (e) {
                  globalThis.__rxjsError = String(e && (e.stack || e.message) || e);
                }
                0;
                "#,
            )
            .expect("RxJS pipeline must run without throwing");
        bridge.runtime_mut().drain_microtasks().unwrap();

        assert_eq!(
            bridge
                .runtime_mut()
                .eval("JSON.stringify(__rxjsValues)")
                .unwrap(),
            JsValue::String("[2,4,6]".to_string()),
            "RxJS of/map/subscribe must produce 2,4,6"
        );
        assert_eq!(
            bridge.runtime_mut().eval("__rxjsComplete").unwrap(),
            JsValue::Boolean(true),
            "RxJS subscription must complete"
        );
        assert_eq!(
            bridge.runtime_mut().eval("__rxjsError").unwrap(),
            JsValue::Null,
            "RxJS pipeline must not error"
        );
    }
}
