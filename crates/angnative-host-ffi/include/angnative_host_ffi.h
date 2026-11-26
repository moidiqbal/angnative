/**
 * angnative_host_ffi.h — C ABI header for AngNative host.
 *
 * Platform shells (Android Kotlin/JNI, iOS Swift, Desktop) include this
 * header to call into the Rust host via FFI.
 *
 * Generated from crates/angnative-host-ffi/src/ffi.rs
 */

#ifndef ANGNATIVE_HOST_FFI_H
#define ANGNATIVE_HOST_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* =========================================================================
 * Opaque handle type
 * ========================================================================= */

/** Opaque handle to a Rust host instance. */
typedef struct AngNativeHost AngNativeHost;

/** Validated handle to the unified runtime bridge (Host + JS engine + runtime).
 * 0 is invalid. The value carries a generation counter so stale handles are
 * rejected before any dereference. */
typedef uint64_t AngNativeRuntimeBridgeHandle;

/* =========================================================================
 * Lifecycle
 * ========================================================================= */

/** Create a new host instance. Returns NULL on failure. */
AngNativeHost* angnative_host_create(void);

/** Destroy a host instance. Safe to call with NULL. */
void angnative_host_destroy(AngNativeHost* handle);

/** Free a string previously allocated by an FFI function. */
void angnative_host_free_string(char* ptr);

/** Get the last error as a JSON string, or {"ok":true} if none.
 * Returns a newly allocated string (caller must free). */
const char* angnative_host_last_error(AngNativeHost* handle);

/* =========================================================================
 * View factory registration
 * ========================================================================= */

/** Callback function pointer types for the ViewFactory trait. */
typedef void (*CreateViewCb)(uint64_t node_id, const char* view_type, void* user_data);
typedef void (*SetFrameCb)(uint64_t node_id, float x, float y, float width, float height, void* user_data);
typedef void (*SetPropsCb)(uint64_t node_id, const char* props_json, void* user_data);
typedef void (*SetTextCb)(uint64_t node_id, const char* text, void* user_data);
typedef void (*AppendChildCb)(uint64_t parent_id, uint64_t child_id, void* user_data);
typedef void (*InsertChildCb)(uint64_t parent_id, uint64_t child_id, uint32_t index, void* user_data);
typedef void (*RemoveChildCb)(uint64_t parent_id, uint64_t child_id, void* user_data);
typedef void (*DestroyViewCb)(uint64_t node_id, void* user_data);

/** Register C function pointers implementing the ViewFactory trait.
 * Returns 0 on success, non-zero on failure. */
int angnative_host_register_view_factory(
    AngNativeHost* handle,
    CreateViewCb create_view,
    SetFrameCb set_frame,
    SetPropsCb set_props,
    SetTextCb set_text,
    AppendChildCb append_child,
    InsertChildCb insert_child,
    RemoveChildCb remove_child,
    DestroyViewCb destroy_view,
    void* user_data
);

/* =========================================================================
 * Command batch processing
 * ========================================================================= */

/** Process a command batch JSON and return platform ops JSON.
 *
 * batch_json - UTF-8 JSON of a CommandBatch
 * out_len    - output length of returned string (optional)
 * Returns: allocated C string with JSON of Vec<PlatformOp>, or NULL on error.
 *          Caller must free with angnative_host_free_string. */
const char* angnative_host_process_batch(
    AngNativeHost* handle,
    const char* batch_json,
    unsigned int* out_len
);

/** Process a raw CommandBatch (in-memory). */
const char* angnative_host_process_batch_raw(
    AngNativeHost* handle,
    void* batch_ptr,
    unsigned int* out_len
);

/* =========================================================================
 * Layout
 * ========================================================================= */

/** Run layout and return geometry rects as JSON.
 *
 * request_json - UTF-8 JSON of a LayoutRequestJson
 * out_len      - output length of returned string (optional)
 * Returns: allocated C string with JSON of LayoutResponseJson, or NULL. */
const char* angnative_host_run_layout(
    AngNativeHost* handle,
    const char* request_json,
    unsigned int* out_len
);

/* =========================================================================
 * Tree inspection
 * ========================================================================= */

/** Get the number of live nodes in the shadow tree. */
unsigned int angnative_host_node_count(AngNativeHost* handle);

/** Serialize the full shadow tree snapshot as JSON. */
const char* angnative_host_snapshot(AngNativeHost* handle);

/* =========================================================================
 * Utility
 * ========================================================================= */

/** Parse a JSON string (for testing). */
const char* angnative_host_parse_json(const char* json);

/* =========================================================================
 * Native event bridge — route platform events to JS/Angular listeners
 * ========================================================================= */

/** Callback delivering a native event to the JS/Angular listener layer. */
typedef void (*EventCallback)(uint64_t listener_id, const char* payload_json, void* user_data);

/** Install the event callback. Returns 0 on success. */
int angnative_host_set_event_callback(AngNativeHost* handle, EventCallback callback, void* user_data);

/** Register a listener id for a (node_id, event_name) pair. Returns 0 on success. */
int angnative_host_add_event_listener(AngNativeHost* handle, uint64_t node_id,
                                      const char* event_name, uint64_t listener_id);

/** Remove a listener. Returns 0 when removed, 1 otherwise. */
int angnative_host_remove_event_listener(AngNativeHost* handle, uint64_t node_id,
                                         const char* event_name, uint64_t listener_id);

/** Dispatch an event to all matching listeners; returns the number invoked. */
unsigned int angnative_host_dispatch_event(AngNativeHost* handle, uint64_t node_id,
                                           const char* event_name, const char* payload_json);

/* =========================================================================
 * Unified runtime bridge — single owner of Host + QuickJS + Angular runtime
 * ========================================================================= */

/** Create a runtime bridge. Returns 0 on failure. The bridge is thread-affine
 * to the creating thread: every other entry point below rejects calls from any
 * other thread without unwinding (Android release builds use panic="abort",
 * where a caught panic would still terminate the process). */
AngNativeRuntimeBridgeHandle angnative_runtime_bridge_create(void);

/** Destroy a runtime bridge (and the Host it owns). Safe to call with 0 or a
 * stale handle. A call from a non-owner thread is a no-op (the bridge is leaked
 * rather than torn down cross-thread); always destroy from the creating thread. */
void angnative_runtime_bridge_destroy(AngNativeRuntimeBridgeHandle handle);

/** Dispatch a native event using one delivery per event (canonical JS thread-local path + legacy FFI fallback).
 * Returns the number of JS listeners invoked (0 on error / no match).
 * Returns UINT_MAX (DISPATCH_PANIC) when called from a non-owner thread; no
 * QuickJS code runs in that case. */
unsigned int angnative_runtime_bridge_dispatch_event(AngNativeRuntimeBridgeHandle handle,
                                                    uint64_t node_id,
                                                    const char* event_name,
                                                    const char* payload_json);

/** Load a raw JS source string (e.g. a self-contained AOT bundle) and drain
 * its microtasks. Returns 0 on success, 1 on invalid input or script error,
 * 2 when called from a non-owner thread. */
int angnative_runtime_bridge_load_script(AngNativeRuntimeBridgeHandle handle,
                                         const char* source);

/** Borrow the Host owned by the bridge (valid until the bridge is destroyed).
 * Returns NULL when called from a non-owner thread. */
AngNativeHost* angnative_runtime_bridge_get_host(AngNativeRuntimeBridgeHandle handle);

/** Lifecycle state: -1 invalid/stale/wrong thread, 0 Created, 1 Initializing,
 * 2 Ready, 3 Running, 4 Suspended, 5 Stopping, 6 Destroyed, 7 Failed. */
int32_t angnative_runtime_bridge_lifecycle_state(AngNativeRuntimeBridgeHandle handle);
int32_t angnative_runtime_bridge_suspend(AngNativeRuntimeBridgeHandle handle);
int32_t angnative_runtime_bridge_resume(AngNativeRuntimeBridgeHandle handle);
int32_t angnative_runtime_bridge_stop(AngNativeRuntimeBridgeHandle handle);

/** Runtime metrics and the last structured error, as newly allocated JSON
 * strings. Free them with angnative_runtime_bridge_free_string. */
char* angnative_runtime_bridge_metrics_json(AngNativeRuntimeBridgeHandle handle);
char* angnative_runtime_bridge_last_error_json(AngNativeRuntimeBridgeHandle handle);

/** Take the pending host-capability tasks queued by JS (async fetch starts,
 * timer schedules and timer cancels) as a newly allocated JSON string. Returns
 * NULL when there is nothing to do. The owner thread must call this; free the
 * result with angnative_runtime_bridge_free_string. */
char* angnative_runtime_bridge_take_host_tasks(AngNativeRuntimeBridgeHandle handle);

void angnative_runtime_bridge_free_string(char* ptr);

/** Optional native callback for structured runtime errors. */
typedef void (*AngNativeErrorCallback)(const char* message,
                                       const char* stack,
                                       void* user_data);
void angnative_runtime_bridge_set_error_callback(
    AngNativeRuntimeBridgeHandle handle,
    AngNativeErrorCallback callback,
    void* user_data);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ANGNATIVE_HOST_FFI_H */
