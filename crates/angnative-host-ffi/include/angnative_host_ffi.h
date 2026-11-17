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

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ANGNATIVE_HOST_FFI_H */
