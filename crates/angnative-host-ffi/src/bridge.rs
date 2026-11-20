/**
 * bridge.rs — Host struct and C-callback ViewFactory adapter.
 *
 * The `Host` owns a MutationEngine, OpEmitter, and optional LayoutEngine,
 * plus an optional ViewFactory implemented via C function pointers.
 *
 * JSON is the wire format across the FFI boundary:
 *   - CommandBatch in → PlatformOp[] out (process_batch)
 *   - LayoutNodeInput[] + dims in → LayoutResult out (run_layout)
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{c_char, CString};
use std::os::raw::c_void;

/// Callback invoked by the FFI event bridge when a native event is dispatched.
/// `listener_id` identifies the registered Angular listener, `payload` is a
/// UTF-8 JSON string owned by the host for the duration of the call.
pub type EventCallback = unsafe extern "C" fn(u64, *const c_char, *mut c_void);

use angnative_host_core::command::CommandBatch;
use angnative_host_core::mutation::MutationEngine;
use angnative_host_layout::{LayoutEngine, LayoutNodeInput};
use angnative_host_view::op_emitter::OpEmitter;
use angnative_host_view::ops::ViewProps;
use angnative_host_view::view_factory::{ViewFactory, ViewFactoryError, ViewFactoryResult};
use angnative_host_view::view_tree::ViewTree;

/* =========================================================================
 * CallbackViewFactory — wraps C function pointers as a ViewFactory
 * ========================================================================= */

/** Opaque user data pointer passed to every callback. */
pub type FfiUserData = *mut c_void;

/** Create a native view. */
pub type CreateViewCb =
    extern "C" fn(node_id: u64, view_type: *const std::ffi::c_char, user_data: FfiUserData);

/** Set the frame of a native view. */
pub type SetFrameCb =
    extern "C" fn(node_id: u64, x: f32, y: f32, width: f32, height: f32, user_data: FfiUserData);

/** Set properties/styles on a native view (JSON payload). */
pub type SetPropsCb =
    extern "C" fn(node_id: u64, props_json: *const std::ffi::c_char, user_data: FfiUserData);

/** Set text content on a native view. */
pub type SetTextCb =
    extern "C" fn(node_id: u64, text: *const std::ffi::c_char, user_data: FfiUserData);

/** Append a child view to a parent. */
pub type AppendChildCb = extern "C" fn(parent_id: u64, child_id: u64, user_data: FfiUserData);

/** Insert a child view at an index. */
pub type InsertChildCb =
    extern "C" fn(parent_id: u64, child_id: u64, index: u32, user_data: FfiUserData);

/** Remove a child view from a parent. */
pub type RemoveChildCb = extern "C" fn(parent_id: u64, child_id: u64, user_data: FfiUserData);

/** Destroy a native view. */
pub type DestroyViewCb = extern "C" fn(node_id: u64, user_data: FfiUserData);

/** All callbacks + user data that a platform shell provides. */
#[derive(Debug)]
pub struct ViewFactoryCallbacks {
    pub create_view: Option<CreateViewCb>,
    pub set_frame: Option<SetFrameCb>,
    pub set_props: Option<SetPropsCb>,
    pub set_text: Option<SetTextCb>,
    pub append_child: Option<AppendChildCb>,
    pub insert_child: Option<InsertChildCb>,
    pub remove_child: Option<RemoveChildCb>,
    pub destroy_view: Option<DestroyViewCb>,
    pub user_data: FfiUserData,
}

impl Default for ViewFactoryCallbacks {
    fn default() -> Self {
        Self {
            create_view: None,
            set_frame: None,
            set_props: None,
            set_text: None,
            append_child: None,
            insert_child: None,
            remove_child: None,
            destroy_view: None,
            user_data: std::ptr::null_mut(),
        }
    }
}

/// ViewFactory implementation that delegates to C function pointers.
/// Platform shells populate ViewFactoryCallbacks and register them via
/// `angnative_host_register_view_factory`.
#[derive(Debug)]
pub struct CallbackViewFactory {
    cb: ViewFactoryCallbacks,
}

// Safety: The Host is used from a single thread per instance (the platform
// shell's main thread). Callbacks and user_data are only accessed on that
// thread, so it is safe to share CallbackViewFactory across threads.
unsafe impl Send for ViewFactoryCallbacks {}
unsafe impl Sync for ViewFactoryCallbacks {}
unsafe impl Send for CallbackViewFactory {}
unsafe impl Sync for CallbackViewFactory {}

impl CallbackViewFactory {
    pub fn new(cb: ViewFactoryCallbacks) -> Self {
        Self { cb }
    }
}

impl ViewFactory for CallbackViewFactory {
    fn create_view(&self, node_id: u64, view_type: &str) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.create_view {
            let c_view_type = std::ffi::CString::new(view_type).unwrap_or_default();
            cb(node_id, c_view_type.as_ptr(), self.cb.user_data);
        }
        Ok(())
    }

    fn set_frame(
        &self,
        node_id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.set_frame {
            cb(node_id, x, y, width, height, self.cb.user_data);
        }
        Ok(())
    }

    fn set_props(&self, node_id: u64, props: &ViewProps) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.set_props {
            let json = serde_json::to_string(props).map_err(|e| {
                ViewFactoryError::OperationFailed(format!("props serialization: {e}"))
            })?;
            let c_json = std::ffi::CString::new(json).unwrap_or_default();
            cb(node_id, c_json.as_ptr(), self.cb.user_data);
        }
        Ok(())
    }

    fn set_text(&self, node_id: u64, text: &str) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.set_text {
            let c_text = std::ffi::CString::new(text).unwrap_or_default();
            cb(node_id, c_text.as_ptr(), self.cb.user_data);
        }
        Ok(())
    }

    fn append_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.append_child {
            cb(parent_id, child_id, self.cb.user_data);
        }
        Ok(())
    }

    fn insert_child(&self, parent_id: u64, child_id: u64, index: usize) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.insert_child {
            cb(parent_id, child_id, index as u32, self.cb.user_data);
        }
        Ok(())
    }

    fn remove_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.remove_child {
            cb(parent_id, child_id, self.cb.user_data);
        }
        Ok(())
    }

    fn destroy_view(&self, node_id: u64) -> ViewFactoryResult<()> {
        if let Some(cb) = self.cb.destroy_view {
            cb(node_id, self.cb.user_data);
        }
        Ok(())
    }
}

/* =========================================================================
 * JSON request/response types for the FFI boundary
 * ========================================================================= */

/** Request to run layout. Mirror of LayoutNodeInput for JSON. */
#[derive(Debug, Deserialize, Serialize)]
pub struct LayoutNodeInputJson {
    pub node_id: u64,
    pub styles: HashMap<String, String>,
    pub children: Vec<u64>,
}

/** Full layout request. */
#[derive(Debug, Deserialize, Serialize)]
pub struct LayoutRequestJson {
    pub root_id: u64,
    pub nodes: Vec<LayoutNodeInputJson>,
    pub available_width: f32,
    pub available_height: f32,
}

/** Response from layout — mirrors LayoutResult. */
#[derive(Debug, Serialize)]
pub struct LayoutResponseJson {
    pub nodes: Vec<angnative_host_layout::LayoutNodeRect>,
    pub content_width: f32,
    pub content_height: f32,
}

/** Generic error response. */
#[derive(Debug, Serialize)]
pub struct ErrorResponseJson {
    pub error: String,
}

/** Generic OK response. */
#[derive(Debug, Serialize)]
pub struct OkResponseJson {
    pub ok: bool,
}

/* =========================================================================
 * Host — owns the Rust host crates + optional ViewFactory
 * ========================================================================= */

/** The FFI host handle. Opaque to platform shells. */
pub struct Host {
    engine: MutationEngine,
    emitter: OpEmitter,
    tree: ViewTree,
    view_factory: Option<Box<dyn ViewFactory>>,
    layout_engine: Option<LayoutEngine>,
    last_error: Option<String>,
    /// Native event bridge: platform shells dispatch events here and the
    /// registered callback forwards them to the JS/Angular listener layer.
    event_callback: Option<EventCallback>,
    event_user_data: *mut c_void,
    event_listeners: HashMap<(u64, String), Vec<u64>>,
}

impl Host {
    pub fn new() -> Self {
        Self {
            engine: MutationEngine::new(),
            emitter: OpEmitter::new(),
            tree: ViewTree::new(),
            view_factory: None,
            layout_engine: Some(LayoutEngine::default()),
            last_error: None,
            event_callback: None,
            event_user_data: std::ptr::null_mut(),
            event_listeners: HashMap::new(),
        }
    }

    pub fn set_last_error(&mut self, msg: impl Into<String>) {
        self.last_error = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Register a ViewFactory implemented via C callbacks.
    pub fn register_view_factory(&mut self, cb: ViewFactoryCallbacks) {
        self.view_factory = Some(Box::new(CallbackViewFactory::new(cb)));
    }

    /* =====================================================================
     * Native event bridge (Angular listener round trip)
     * ===================================================================== */

    /// Install the single callback used to deliver native events to JS/Angular.
    pub fn set_event_callback(&mut self, cb: EventCallback, user_data: *mut c_void) {
        self.event_callback = Some(cb);
        self.event_user_data = user_data;
    }

    /// Register a listener id for `(node_id, event_name)`.
    pub fn add_event_listener(&mut self, node_id: u64, event_name: &str, listener_id: u64) {
        self.event_listeners
            .entry((node_id, event_name.to_string()))
            .or_default()
            .push(listener_id);
    }

    /// Remove a previously registered listener. Returns true when removed.
    pub fn remove_event_listener(
        &mut self,
        node_id: u64,
        event_name: &str,
        listener_id: u64,
    ) -> bool {
        let key = (node_id, event_name.to_string());
        let mut removed = false;
        if let Some(ids) = self.event_listeners.get_mut(&key) {
            if let Some(pos) = ids.iter().position(|id| *id == listener_id) {
                ids.remove(pos);
                removed = true;
            }
            if ids.is_empty() {
                self.event_listeners.remove(&key);
            }
        }
        removed
    }

    /// Dispatch a native event to every registered listener. Returns how many
    /// listeners were invoked (0 also when no callback is installed).
    pub fn dispatch_event(&mut self, node_id: u64, event_name: &str, payload: &str) -> u32 {
        let ids = self
            .event_listeners
            .get(&(node_id, event_name.to_string()))
            .cloned()
            .unwrap_or_default();
        let Some(cb) = self.event_callback else {
            return 0;
        };
        let Ok(c_payload) = CString::new(payload) else {
            return 0;
        };
        let user_data = self.event_user_data;
        let mut invoked = 0;
        for id in ids {
            unsafe { cb(id, c_payload.as_ptr(), user_data) };
            invoked += 1;
        }
        invoked
    }

    /// Process a command batch: decode JSON → validate → emit ops → apply to view factory.
    pub fn process_batch_json(&mut self, batch_json: &str) -> Result<String, String> {
        self.clear_error();

        // Decode the command batch from JSON.
        let batch: CommandBatch = serde_json::from_str(batch_json)
            .map_err(|e| format!("Invalid command batch JSON: {e}"))?;

        // Validate protocol version.
        angnative_host_core::command::validate_version(batch.protocol_version)
            .map_err(|e| format!("Protocol error: {e}"))?;

        // Apply the batch to the retained shadow tree so node_count/snapshot
        // (used by platform shells) reflect the current state.
        self.engine
            .run_batch(batch.clone())
            .map_err(|e| format!("MutationEngine error: {e}"))?;

        // Emit platform ops through the OpEmitter.
        let ops = self
            .emitter
            .process_batch(&batch)
            .map_err(|e| format!("OpEmitter error: {e}"))?;

        // Apply ops to the registered view factory, if any.
        if let Some(factory) = &self.view_factory {
            factory
                .apply_ops(&ops)
                .map_err(|e| format!("ViewFactory error: {e}"))?;
        }

        // Serialize ops back to JSON for the platform shell.
        serde_json::to_string(&ops).map_err(|e| format!("Ops serialization: {e}"))
    }

    /// Run layout: decode nodes JSON → compute → return rects JSON.
    pub fn run_layout_json(&mut self, request_json: &str) -> Result<String, String> {
        self.clear_error();

        let request: LayoutRequestJson =
            serde_json::from_str(request_json).map_err(|e| format!("Invalid layout JSON: {e}"))?;

        let layout_engine = self
            .layout_engine
            .as_mut()
            .ok_or_else(|| "Layout engine not initialized".to_string())?;

        // Convert JSON inputs to LayoutNodeInput.
        let mut nodes = HashMap::new();
        for input in &request.nodes {
            nodes.insert(
                input.node_id,
                LayoutNodeInput {
                    node_id: input.node_id,
                    styles: input.styles.clone(),
                    children: input.children.clone(),
                },
            );
        }

        let result = layout_engine.layout(
            request.root_id,
            &nodes,
            request.available_width,
            request.available_height,
        )?;

        let response = LayoutResponseJson {
            nodes: result.nodes,
            content_width: result.content_size.width,
            content_height: result.content_size.height,
        };

        serde_json::to_string(&response).map_err(|e| format!("Layout serialization: {e}"))
    }

    /// Get the last error as JSON, or `{"ok":true}` if none.
    /// Reading the error does not clear it; `clear_error` does.
    pub fn last_error_json(&mut self) -> String {
        match self.last_error.as_ref() {
            Some(msg) => serde_json::to_string(&ErrorResponseJson { error: msg.clone() }).unwrap(),
            None => serde_json::to_string(&OkResponseJson { ok: true }).unwrap(),
        }
    }

    /// Expose the mutation engine for tree-state queries (used by conformance).
    pub fn engine(&mut self) -> &mut MutationEngine {
        &mut self.engine
    }

    /// Expose the op emitter for inspection.
    pub fn emitter(&self) -> &OpEmitter {
        &self.emitter
    }

    /// Expose the op emitter mutably.
    pub fn emitter_mut(&mut self) -> &mut OpEmitter {
        &mut self.emitter
    }

    /// Expose the view tree for inspection.
    pub fn tree(&self) -> &ViewTree {
        &self.tree
    }
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}
