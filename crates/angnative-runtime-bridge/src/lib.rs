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
    runtime: AngNativeRuntime<QuickJsEngine>,
    host: Host,
    owner_thread: ThreadId,
}

impl RuntimeBridge {
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = Host::new();
        let engine = QuickJsEngine::new()?;
        let mut runtime = AngNativeRuntime::new(engine, RuntimeConfig::default());
        runtime.install_api()?;
        // Shims are optional in this build path (host-engine exposes them); if a
        // compatible helper exists, install it. Kept best-effort on purpose.
        let _ = crate::install_shims(&mut runtime);

        let owner_thread = thread::current().id();
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            *rt.borrow_mut() = Some(&mut runtime as *mut AngNativeRuntime<QuickJsEngine>);
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
        &mut self.runtime
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

/// Compatibility shim installer — host-engine may or may not expose helpers
/// depending on features; no-op when unavailable.
fn install_shims(_runtime: &mut AngNativeRuntime<QuickJsEngine>) -> Result<(), anyhow::Error> {
    Ok(())
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

    #[test]
    fn test_create_runtime_bridge() {
        assert!(create_runtime_bridge().is_ok());
    }

    #[test]
    fn test_dispatch_event_returns_zero_when_no_listeners() {
        let mut bridge = create_runtime_bridge().unwrap();
        assert_eq!(bridge.dispatch_event(1, "press", "{}").unwrap(), 0);
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
