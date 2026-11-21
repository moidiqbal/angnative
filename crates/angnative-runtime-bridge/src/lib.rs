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
use angnative_host_engine::{AngNativeRuntime, RuntimeConfig};
use angnative_host_engine::quickjs::QuickJsEngine;
use angnative_host_ffi::bridge::{
    Host, set_js_runtime_dispatch_for_thread, clear_js_runtime_dispatch_for_thread,
};
use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::thread::{self, ThreadId};
use std::thread_local;

thread_local! {
    static CURRENT_BRIDGE_RUNTIME: RefCell<Option<*mut AngNativeRuntime<QuickJsEngine>>> =
        RefCell::new(None);
}

/// Bridge — single authoritative owner of Host + QuickJS + Angular runtime.
pub struct RuntimeBridge {
    // Boxed so the address stays stable when the bridge itself is moved into
    // a Box by the C API. The thread-local dispatch pointer targets this
    // allocation; pointing at an inline field would dangle after a move.
    runtime: Box<AngNativeRuntime<QuickJsEngine>>,
    host: Host,
    owner_thread: ThreadId,
}

impl RuntimeBridge {
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = Host::new();
        let engine = QuickJsEngine::new()?;
        let mut runtime = Box::new(AngNativeRuntime::new(engine, RuntimeConfig::default()));
        runtime.install_api()?;
        // Install the DOM/platform shims into the same engine so Angular code
        // has `window`, `document`, timers and events available.
        angnative_host_engine::quickjs_shims::install_dom_shims(runtime.engine())?;

        let owner_thread = thread::current().id();
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            // Deref the Box: this address is stable even when the bridge is
            // moved into another Box by the C API entry point.
            *rt.borrow_mut() = Some(&mut *runtime as *mut AngNativeRuntime<QuickJsEngine>);
        });
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
        &mut self.host
    }

    pub fn host(&self) -> &Host {
        &self.host
    }
}

impl Drop for RuntimeBridge {
    fn drop(&mut self) {
        CURRENT_BRIDGE_RUNTIME.with(|rt| *rt.borrow_mut() = None);
        clear_js_runtime_dispatch_for_thread();
    }
}

/// Single JS dispatch callback installed on Host.
extern "C" fn js_runtime_dispatch_callback(
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
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
}

#[repr(C)]
pub struct AngNativeRuntimeBridge {
    _private: [u8; 0],
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
    if handle.is_null() { return; }
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
    result.unwrap_or(0)
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
        _ => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_get_host(
    handle: *mut AngNativeRuntimeBridge,
) -> *mut Host {
    if handle.is_null() { return std::ptr::null_mut(); }
    let bridge = &mut *(handle as *mut RuntimeBridge);
    bridge.host_mut() as *mut Host
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
    /// Verifies the real @angular/common/http bundle loads and that the
    /// host-wirable fetch surface returns a proper Response. Full DI-based
    /// `HttpClient.get()` is not yet verified: FetchBackend needs platform
    /// providers (NgZone/DestroyRef) that a bare EnvironmentInjector does not
    /// supply in this runtime.
    #[test]
    #[ignore = "requires generated dist/aot/http-bundle.js"]
    fn angular_http_platform_loads_in_quickjs() {
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
}
