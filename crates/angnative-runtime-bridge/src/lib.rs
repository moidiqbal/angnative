/**
 * lib.rs — Runtime bridge connecting Host event callbacks to AngNativeRuntime.
 *
 * This crate wires the FFI Host's native event callback to the QuickJS
 * AngNativeRuntime, enabling the full Angular (click) round trip:
 *
 *   native button click → Platform shell calls RuntimeBridge::dispatch_event
 *   → AngNativeRuntime::dispatch_event → JS na_onEvent(listenerId, payload)
 *   → Angular Renderer2 listener → component handler runs
 *
 * Ownership model:
 * - RuntimeBridge owns the Host, QuickJsEngine, and AngNativeRuntime
 * - QuickJsEngine is thread-affine and NOT Send/Sync - it lives on the creating thread
 * - RuntimeBridge is the single authoritative instance per application
 * - Platform shells hold an opaque handle and call C-API functions
 * - NO global singletons, NO unsafe transmute, NO duplicate event paths
 * - Thread-local storage used for JS runtime dispatch callback
 */
use angnative_host_engine::{AngNativeRuntime, RuntimeConfig};
use angnative_host_engine::quickjs::QuickJsEngine;
use angnative_host_ffi::bridge::{Host, set_js_runtime_dispatch_for_thread, clear_js_runtime_dispatch_for_thread};
use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::os::raw::c_int;
use std::thread_local;

/// Thread-local storage for the current bridge's runtime.
/// Allows the C callback to access the AngNativeRuntime.
thread_local! {
    static CURRENT_BRIDGE_RUNTIME: RefCell<Option<*mut AngNativeRuntime<QuickJsEngine>>> = RefCell::new(None);
}

/// Bridge that connects a Host's event callback to an AngNativeRuntime.
///
/// The bridge is the single owner of:
/// - Host (MutationEngine, OpEmitter, ViewTree, event registry)
/// - QuickJsEngine (thread-affine, not Send/Sync)
/// - AngNativeRuntime (JS API + event dispatch)
///
/// Platform shells must call C-API functions on the same thread that created the bridge.
pub struct RuntimeBridge {
    runtime: AngNativeRuntime<QuickJsEngine>,
    host: Host,
}

impl RuntimeBridge {
    /// Create a new bridge.
    ///
    /// Creates a unified Host + QuickJsEngine + AngNativeRuntime + RuntimeBridge.
    /// Installs the `na.*` API into the JS engine.
    /// Registers the JS runtime dispatch callback via thread-local storage.
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = Host::new();
        let engine = QuickJsEngine::new()?;
        let mut runtime = AngNativeRuntime::new(engine, RuntimeConfig::default());
        runtime.install_api()?;

        // Store runtime pointer in thread-local for C callback access
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            *rt.borrow_mut() = Some(&mut runtime as *mut AngNativeRuntime<QuickJsEngine>);
        });

        // Set the thread-local dispatch function
        set_js_runtime_dispatch_for_thread(js_runtime_dispatch_callback);

        Ok(Self { runtime, host })
    }

    /// Dispatch a native event to the JS runtime.
    ///
    /// This is the SINGLE authoritative event dispatch path.
    /// Calls AngNativeRuntime::dispatch_event which invokes JS `na_onEvent` for each listener.
    /// Returns the number of JS listeners invoked.
    pub fn dispatch_event(
        &mut self,
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> Result<u32, anyhow::Error> {
        Ok(self.runtime.dispatch_event(node_id, event_name, payload_json)?)
    }

    /// Get mutable access to the runtime for eval, module loading, etc.
    pub fn runtime_mut(&mut self) -> &mut AngNativeRuntime<QuickJsEngine> {
        &mut self.runtime
    }

    /// Get mutable access to the host for process_batch, layout, etc.
    pub fn host_mut(&mut self) -> &mut Host {
        &mut self.host
    }

    /// Get immutable access to the host for inspection.
    pub fn host(&self) -> &Host {
        &self.host
    }
}

impl Drop for RuntimeBridge {
    fn drop(&mut self) {
        // Clear thread-local on drop
        CURRENT_BRIDGE_RUNTIME.with(|rt| *rt.borrow_mut() = None);
        clear_js_runtime_dispatch_for_thread();
    }
}

/// C callback function for Host's JS runtime dispatch.
/// Called by Host::dispatch_event when a native event occurs.
/// Accesses the current bridge's runtime via thread-local storage.
extern "C" fn js_runtime_dispatch_callback(
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
    // Convert C strings to Rust strings
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

    // Get the current bridge's runtime from thread-local
    CURRENT_BRIDGE_RUNTIME.with(|rt| {
        if let Some(runtime_ptr) = rt.borrow().as_ref() {
            // SAFETY: RuntimeBridge owns the runtime and lives on this thread
            let runtime = unsafe { &mut **runtime_ptr };
            match runtime.dispatch_event(node_id, event_name_str, payload_str) {
                Ok(invoked) => invoked,
                Err(_) => 0,
            }
        } else {
            0
        }
    })
}

/// Opaque handle for the runtime bridge.
#[repr(C)]
pub struct AngNativeRuntimeBridge {
    _private: [u8; 0],
}

/// Create a new runtime bridge. Returns an opaque handle.
///
/// This creates: Host + QuickJsEngine + AngNativeRuntime + RuntimeBridge
/// Installs the `na.*` API into the JS engine.
/// Registers JS runtime dispatch as the single event path.
#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_create() -> *mut AngNativeRuntimeBridge {
    match std::panic::catch_unwind(|| {
        let bridge = RuntimeBridge::new();
        match bridge {
            Ok(b) => Box::into_raw(Box::new(b)) as *mut AngNativeRuntimeBridge,
            Err(_) => std::ptr::null_mut(),
        }
    }) {
        Ok(ptr) => ptr,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Destroy a runtime bridge.
#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_destroy(handle: *mut AngNativeRuntimeBridge) {
    if handle.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(|| {
        let _bridge = Box::from_raw(handle as *mut RuntimeBridge);
    });
}

/// Dispatch a native event through the runtime bridge.
///
/// This is the function platform shells (Android, iOS, Desktop) should call
/// when a native button is pressed, instead of the current imperative
/// `SetTextContent` batch approach.
///
/// Returns the number of JS listeners invoked.
#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_dispatch_event(
    handle: *mut AngNativeRuntimeBridge,
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
    if handle.is_null() || event_name.is_null() {
        return 0;
    }
    let bridge = &mut *(handle as *mut RuntimeBridge);
    let event_name_str = match CStr::from_ptr(event_name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let payload_str = if payload_json.is_null() {
        "{}"
    } else {
        match CStr::from_ptr(payload_json).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };
    match bridge.dispatch_event(node_id, event_name_str, payload_str) {
        Ok(invoked) => invoked,
        Err(_) => 0,
    }
}

/// Get the underlying host pointer for process_batch, layout, etc.
/// 
/// This allows platform shells to use the same Host for both
/// ViewFactory operations AND command batch processing.
#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_get_host(
    handle: *mut AngNativeRuntimeBridge,
) -> *mut angnative_host_ffi::bridge::Host {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let bridge = &mut *(handle as *mut RuntimeBridge);
    bridge.host_mut() as *mut Host
}

/// Convenience function to create a complete runtime setup.
pub fn create_runtime_bridge() -> Result<RuntimeBridge, anyhow::Error> {
    RuntimeBridge::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_runtime_bridge() {
        let bridge = create_runtime_bridge();
        assert!(bridge.is_ok());
    }

    #[test]
    fn test_dispatch_event_returns_zero_when_no_listeners() {
        let mut bridge = create_runtime_bridge().unwrap();
        let result = bridge.dispatch_event(1, "press", "{}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}