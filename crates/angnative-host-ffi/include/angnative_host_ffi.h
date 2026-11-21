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

/** Opaque handle to the unified runtime bridge (Host + QuickJS + runtime). */
typedef struct AngNativeRuntimeBridge AngNativeRuntimeBridge;

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

/** Create a runtime bridge. Returns NULL on failure. Must be used on one thread. */
AngNativeRuntimeBridge* angnative_runtime_bridge_create(void);

/** Destroy a runtime bridge (and the Host it owns). Safe to call with NULL. */
void angnative_runtime_bridge_destroy(AngNativeRuntimeBridge* handle);

/** Dispatch a native event through the single JS event pipeline.
 * Returns the number of JS listeners invoked (0 on error / no match). */
unsigned int angnative_runtime_bridge_dispatch_event(AngNativeRuntimeBridge* handle,
                                                    uint64_t node_id,
                                                    const char* event_name,
                                                    const char* payload_json);

/** Load a raw JS source string (e.g. a self-contained AOT bundle) and drain
 * its microtasks. Returns 0 on success, 1 on invalid input or script error. */
int angnative_runtime_bridge_load_script(AngNativeRuntimeBridge* handle,
                                         const char* source);

/** Borrow the Host owned by the bridge (valid until the bridge is destroyed). */
AngNativeHost* angnative_runtime_bridge_get_host(AngNativeRuntimeBridge* handle);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ANGNATIVE_HOST_FFI_H */
