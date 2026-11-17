/**
 * ffi.rs — Stable C ABI entry points for angnative-host-ffi.
 *
 * Platform shells (Android Kotlin via JNI, iOS Swift, Desktop) call these
 * functions through FFI. All strings are UTF-8, null-terminated, and owned
 * by the caller until freed with `angnative_host_free_string`.
 *
 * Opaque handle: `AngNativeHost` (alias for `*mut Host`). Platform shells
 * never dereference it — they only pass it back.
 *
 * Error handling: every function returns `i32`. `0` means success, non-zero
 * means failure. Use `angnative_host_last_error()` to retrieve a JSON error
 * message after a failure.
 */

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use crate::bridge::{
    Host, ViewFactoryCallbacks,
};
use angnative_host_core::command::CommandBatch;
use serde_json::Value;

/* =========================================================================
 * Opaque handle type
 * ========================================================================= */

/** Opaque handle to a Rust host instance. Platform shells treat this as `void*`. */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AngNativeHost {
    _private: [u8; 0],
}

/* =========================================================================
 * Lifecycle
 * ========================================================================= */

/** Create a new host instance. Returns a non-null handle on success, null on failure. */
#[no_mangle]
pub extern "C" fn angnative_host_create() -> *mut AngNativeHost {
    match std::panic::catch_unwind(|| {
        let host = Box::new(Host::new());
        Box::into_raw(host) as *mut AngNativeHost
    }) {
        Ok(ptr) => ptr,
        Err(_) => std::ptr::null_mut(),
    }
}

/** Destroy a host instance and free all associated memory. Safe to call with null. */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_destroy(handle: *mut AngNativeHost) {
    if handle.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(|| {
        let _host = Box::from_raw(handle as *mut Host);
        // Host is dropped here, freeing all memory.
    });
}

/** Free a string previously allocated by an FFI function. Safe to call with null. */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(|| {
        let _ = CString::from_raw(ptr);
    });
}

/** Get the last error as a JSON string, or {"ok":true} if none.
 *
 * Returns a newly allocated C string (caller must free with
 * angnative_host_free_string). Returns null on allocation failure.
 */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_last_error(handle: *mut AngNativeHost) -> *mut c_char {
    if handle.is_null() {
        return CString::new(r#"{"error":"null handle"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let host = unsafe { &mut *(handle as *mut Host) };
    let json = host.last_error_json();

    CString::new(json)
        .ok()
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/* =========================================================================
 * View factory registration
 * ========================================================================= */

/** Register C function pointers implementing the ViewFactory trait.
 *
 * All callback pointers are optional — pass a null function pointer for any
 * callback the platform shell does not implement. `user_data` is forwarded
 * unchanged to every callback invocation.
 */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_register_view_factory(
    handle: *mut AngNativeHost,
    create_view: Option<crate::bridge::CreateViewCb>,
    set_frame: Option<crate::bridge::SetFrameCb>,
    set_props: Option<crate::bridge::SetPropsCb>,
    set_text: Option<crate::bridge::SetTextCb>,
    append_child: Option<crate::bridge::AppendChildCb>,
    insert_child: Option<crate::bridge::InsertChildCb>,
    remove_child: Option<crate::bridge::RemoveChildCb>,
    destroy_view: Option<crate::bridge::DestroyViewCb>,
    user_data: *mut c_void,
) -> c_int {
    if handle.is_null() {
        return 1;
    }

    let host = unsafe { &mut *(handle as *mut Host) };

    let cbs = ViewFactoryCallbacks {
        create_view,
        set_frame,
        set_props,
        set_text,
        append_child,
        insert_child,
        remove_child,
        destroy_view,
        user_data,
    };

    host.register_view_factory(cbs);
    0
}

/* =========================================================================
 * Command batch processing
 * ========================================================================= */

/** Process a command batch JSON and return platform ops JSON.
 *
 * `batch_json` — UTF-8 JSON of a CommandBatch (from @angnative/contract).
 * Returns: allocated C string with JSON of Vec<PlatformOp>, or null on error.
 *          Caller frees with angnative_host_free_string.
 */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_process_batch(
    handle: *mut AngNativeHost,
    batch_json: *const c_char,
    out_len: *mut u32,
) -> *mut c_char {
    if handle.is_null() || batch_json.is_null() {
        return CString::new(r#"{"error":"null handle or batch_json"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let host = unsafe { &mut *(handle as *mut Host) };
    let json_str = match unsafe { CStr::from_ptr(batch_json) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            host.set_last_error("batch_json is not valid UTF-8");
            return error_cstring(host);
        }
    };

    match host.process_batch_json(json_str) {
        Ok(ops_json) => {
            if !out_len.is_null() {
                unsafe { *out_len = ops_json.len() as u32 };
            }
            CString::new(ops_json)
                .ok()
                .map(|s| s.into_raw())
                .unwrap_or_else(|| error_cstring(host))
        }
        Err(e) => {
            host.set_last_error(&e);
            error_cstring(host)
        }
    }
}

/** Process a raw CommandBatch (in-memory, no JSON parse).
 *
 * Used by Rust-side consumers that already have a CommandBatch value.
 */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_process_batch_raw(
    handle: *mut AngNativeHost,
    batch_ptr: *mut CommandBatch,
    out_len: *mut u32,
) -> *mut c_char {
    if handle.is_null() || batch_ptr.is_null() {
        return CString::new(r#"{"error":"null handle or batch_ptr"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let host = unsafe { &mut *(handle as *mut Host) };
    let batch = unsafe { &*batch_ptr };

    match host.emitter_mut().process_batch(batch) {
        Ok(ops) => {
            if !out_len.is_null() {
                unsafe { *out_len = ops.len() as u32 };
            }
            match serde_json::to_string(&ops) {
                Ok(json) => CString::new(json)
                    .ok()
                    .map(|s| s.into_raw())
                    .unwrap_or_else(|| error_cstring(host)),
                Err(e) => {
                    host.set_last_error(format!("ops serialization: {e}"));
                    error_cstring(host)
                }
            }
        }
        Err(e) => {
            host.set_last_error(format!("op emitter: {e}"));
            error_cstring(host)
        }
    }
}

/* =========================================================================
 * Layout
 * ========================================================================= */

/** Run layout and return geometry rects as JSON.
 *
 * `request_json` — UTF-8 JSON of a LayoutRequestJson:
 *   { "root_id": u64, "nodes": [{node_id, styles, children}],
 *     "available_width": f32, "available_height": f32 }
 * Returns: allocated C string with JSON of LayoutResponseJson, or null on error.
 */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_run_layout(
    handle: *mut AngNativeHost,
    request_json: *const c_char,
    out_len: *mut u32,
) -> *mut c_char {
    if handle.is_null() || request_json.is_null() {
        return CString::new(r#"{"error":"null handle or request_json"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let host = unsafe { &mut *(handle as *mut Host) };
    let json_str = match unsafe { CStr::from_ptr(request_json) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            host.set_last_error("request_json is not valid UTF-8");
            return error_cstring(host);
        }
    };

    match host.run_layout_json(json_str) {
        Ok(layout_json) => {
            if !out_len.is_null() {
                unsafe { *out_len = layout_json.len() as u32 };
            }
            CString::new(layout_json)
                .ok()
                .map(|s| s.into_raw())
                .unwrap_or_else(|| error_cstring(host))
        }
        Err(e) => {
            host.set_last_error(&e);
            error_cstring(host)
        }
    }
}

/* =========================================================================
 * Tree inspection (conformance / debugging)
 * ========================================================================= */

/** Get the number of live nodes in the shadow tree. */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_node_count(handle: *mut AngNativeHost) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let host = unsafe { &mut *(handle as *mut Host) };
    host.engine().size() as u32
}

/** Serialize the full shadow tree snapshot as JSON. */
#[no_mangle]
pub unsafe extern "C" fn angnative_host_snapshot(handle: *mut AngNativeHost) -> *mut c_char {
    if handle.is_null() {
        return CString::new(r#"{"error":"null handle"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let host = unsafe { &mut *(handle as *mut Host) };
    let snapshot = host.engine().snapshot();

    match serde_json::to_string(&snapshot) {
        Ok(json) => CString::new(json)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or_else(|| error_cstring(host)),
        Err(e) => {
            host.set_last_error(format!("snapshot serialization: {e}"));
            error_cstring(host)
        }
    }
}

/* =========================================================================
 * Helpers
 * ========================================================================= */

fn error_cstring(host: &mut Host) -> *mut c_char {
    let json = host.last_error_json();
    CString::new(json)
        .ok()
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

// Allow C callers to pass a raw JSON Value for advanced use cases.
#[no_mangle]
pub unsafe extern "C" fn angnative_host_parse_json(json: *const c_char) -> *mut c_char {
    if json.is_null() {
        return CString::new(r#"{"error":"null json"}"#)
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }

    let s = match unsafe { CStr::from_ptr(json) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            return CString::new(r#"{"error":"not UTF-8"}"#)
                .ok()
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut());
        }
    };

    let parsed: Result<Value, _> = serde_json::from_str(s);
    match parsed {
        Ok(v) => match serde_json::to_string(&v) {
            Ok(json) => CString::new(json)
                .ok()
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut()),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}
