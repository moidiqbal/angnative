/**
 * lib.rs — Runtime bridge connecting Host event callbacks to AngNativeRuntime.
 *
 * One delivery per event: the canonical JS thread-local path is used by the
 * legacy Host::dispatch_event shell, while the unified FFI/JNI path calls the
 * bridge directly as a fallback. Both paths invoke one JS listener at a time;
 * there is no second listener callback installed on Host.
 *
 * Gate B/F additions (2026-11-23):
 * - validated u64 handles with monotonically increasing generations;
 * - RuntimeController lifecycle integrated into the bridge;
 * - unified AngNativeError, RuntimeMetrics and native error callbacks;
 * - RuntimeBridge generic over any JsEngine (QuickJS remains the C ABI default).
 */
use angnative_host_engine::quickjs::QuickJsEngine;
use angnative_host_engine::{
    AngNativeError, AngNativeRuntime, EngineError, ErrorCode, JsEngine, JsValue, LifecycleState,
    RuntimeConfig, RuntimeController, RuntimeControllerConfig, RuntimeErrorInfo, RuntimeMetrics,
};
use angnative_host_ffi::bridge::{
    clear_js_runtime_dispatch_for_thread, set_js_runtime_dispatch_for_thread, Host,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};
use std::thread_local;
use std::time::Instant;

/// Sentinel returned by event dispatch when a panic was caught / wrong thread.
pub const DISPATCH_PANIC: u32 = u32::MAX;
/// 0 = success, 1 = script error / invalid input, 2 = wrong thread or panic.
pub const LOAD_SCRIPT_PANIC: i32 = 2;

#[cfg(test)]
static WRONG_THREAD_FFI_REJECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Native error callback signature used by the C ABI.
pub type AngNativeErrorCallback = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[derive(Default)]
struct JsErrorState {
    last: Option<(String, Option<String>)>,
    callback: Option<(AngNativeErrorCallback, *mut c_void)>,
}

thread_local! {
    /// Type-erased runtime dispatch, keyed by bridge owner (P1-1). Keeping one
    /// entry per live bridge prevents a second bridge from repointing the
    /// first bridge's event/batch callbacks on the same thread.
    static CURRENT_BRIDGE_RUNTIME: RefCell<HashMap<usize, *mut dyn RuntimeEventDispatch>> =
        RefCell::new(HashMap::new());
    /// Stable pointer to each bridge-owned Host for `na_dispatchBatch`.
    static CURRENT_BRIDGE_HOST: RefCell<HashMap<usize, *mut Host>> =
        RefCell::new(HashMap::new());
    /// Per-bridge native error callback state.
    static JS_ERROR_STATE: RefCell<HashMap<usize, JsErrorState>> =
        RefCell::new(HashMap::new());
}

/// Unique, process-local identity for a live bridge. Never crosses FFI.
static NEXT_BRIDGE_OWNER: AtomicU64 = AtomicU64::new(1);

/// Engine-neutral view of an AngNativeRuntime used by the canonical JS callback.
pub trait RuntimeEventDispatch {
    fn dispatch_event_erased(
        &mut self,
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> Result<u32, AngNativeError>;
}

impl<E: JsEngine> RuntimeEventDispatch for AngNativeRuntime<E> {
    fn dispatch_event_erased(
        &mut self,
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> Result<u32, AngNativeError> {
        Ok(self.dispatch_event(node_id, event_name, payload_json)?)
    }
}

/// Bridge — single authoritative owner of Host + JS engine + Angular runtime.
/// `E` defaults to QuickJS for the C ABI; tests use a reference engine to prove
/// the abstraction (Gate F).
pub struct RuntimeBridge<E: JsEngine = QuickJsEngine> {
    // Boxed so the thread-local dispatch pointer stays stable when the bridge
    // struct is moved (for example into a Box by the handle registry).
    runtime: Box<AngNativeRuntime<E>>,
    host: Box<Host>,
    /// Key for every owner-scoped thread-local slot and callback.
    owner_id: usize,
    owner_thread: ThreadId,
    controller: RuntimeController,
    metrics: RuntimeMetrics,
    last_error: Option<RuntimeErrorInfo>,
    error_callback: Option<AngNativeErrorCallback>,
    error_user_data: *mut c_void,
}

impl std::fmt::Debug for RuntimeBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeBridge")
            .field("owner_thread", &self.owner_thread)
            .field("state", &self.controller.state())
            .field("metrics", &self.metrics)
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl RuntimeBridge<QuickJsEngine> {
    pub fn new() -> Result<Self, AngNativeError> {
        let mut bridge = Self::with_engine(QuickJsEngine::new()?)?;
        angnative_host_engine::quickjs_shims::install_dom_shims(bridge.runtime.engine())?;
        Ok(bridge)
    }
}

impl<E: JsEngine + 'static> RuntimeBridge<E> {
    /// Construct a bridge with any JsEngine. QuickJS-specific DOM shims are
    /// installed by `RuntimeBridge::<QuickJsEngine>::new()` only.
    pub fn with_engine(engine: E) -> Result<Self, AngNativeError> {
        let engine_name = engine.engine_name();
        let heap_bytes = engine.memory_usage();
        let owner_id = NEXT_BRIDGE_OWNER.fetch_add(1, AtomicOrdering::SeqCst) as usize;
        let mut host = Box::new(Host::new());
        host.set_js_dispatch_owner(owner_id);
        let mut runtime = Box::new(AngNativeRuntime::new(engine, RuntimeConfig::default()));
        runtime.install_api()?;

        let controller = RuntimeController::new(RuntimeControllerConfig::default())?;
        controller.initialize()?; // Created -> Initializing -> Ready
        controller.run()?; // Ready -> Running (dispatch remains available)

        let owner_thread = thread::current().id();
        let mut bridge = Self {
            runtime,
            host,
            owner_id,
            owner_thread,
            controller,
            metrics: RuntimeMetrics::new(engine_name, heap_bytes),
            last_error: None,
            error_callback: None,
            error_user_data: std::ptr::null_mut(),
        };

        bridge.install_thread_locals();
        bridge.register_bridge_functions()?;
        set_js_runtime_dispatch_for_thread(bridge.owner_id, js_runtime_dispatch_callback);
        Ok(bridge)
    }

    fn install_thread_locals(&mut self) {
        let dispatch: *mut dyn RuntimeEventDispatch = &mut *self.runtime;
        let host: *mut Host = &mut *self.host;
        let owner = self.owner_id;
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            let previous = rt.borrow_mut().insert(owner, dispatch);
            assert!(previous.is_none(), "bridge owner {owner} already installed");
        });
        CURRENT_BRIDGE_HOST.with(|h| {
            let previous = h.borrow_mut().insert(owner, host);
            assert!(previous.is_none(), "bridge owner {owner} already installed");
        });
        JS_ERROR_STATE.with(|state| {
            let previous = state.borrow_mut().insert(owner, JsErrorState::default());
            assert!(previous.is_none(), "bridge owner {owner} already installed");
        });
    }

    fn register_bridge_functions(&mut self) -> Result<(), AngNativeError> {
        let owner = self.owner_id;

        // Angular renderer batches flow into this bridge's Host only.
        self.runtime.engine().register_function("na_dispatchBatch", move |args| {
            let Some(JsValue::String(json)) = args.first() else {
                return Ok(JsValue::Null);
            };
            // Copy the pointer and drop the RefCell borrow before touching the
            // Host; a batch can synchronously call back into JS (P2-4).
            let host = CURRENT_BRIDGE_HOST.with(|h| h.borrow().get(&owner).copied());
            let Some(host) = host else {
                return Ok(JsValue::Null);
            };
            let host = unsafe { &mut *host };
            match host.process_batch_json(json.as_str()) {
                Ok(ops) => Ok(JsValue::String(ops)),
                Err(e) => Err(EngineError::CallError(e)),
            }
        });

        // Angular ErrorHandler provider calls this global (P1-09). Record only;
        // the single notification point is `drain_js_reported_error` (P2-1).
        self.runtime.engine().register_function("na_reportError", move |args| {
            let message = match args.first() {
                Some(JsValue::String(s)) => s.clone(),
                Some(other) => format!("{other:?}"),
                None => "unknown error".to_string(),
            };
            let stack = match args.get(1) {
                Some(JsValue::String(s)) => Some(s.clone()),
                _ => None,
            };
            JS_ERROR_STATE.with(|state| {
                if let Some(state) = state.borrow_mut().get_mut(&owner) {
                    state.last = Some((message, stack));
                }
            });
            Ok(JsValue::Null)
        });

        self.runtime.engine().eval(
            r#"globalThis.__angnativeOnError = function (error, stack) {
                var message = (error && error.message) ? String(error.message)
                    : (error != null ? String(error) : 'unknown error');
                na_reportError(message, stack == null ? null : String(stack));
            };"#,
        )?;

        Ok(())
    }

    fn assert_owner_thread(&self) {
        assert_eq!(
            thread::current().id(), self.owner_thread,
            "JS runtime must be used on its creating thread"
        );
    }

    fn require_running(&self) -> Result<(), AngNativeError> {
        match self.controller.state() {
            LifecycleState::Running => Ok(()),
            other => Err(AngNativeError::Lifecycle(format!(
                "runtime is {:?}; expected Running",
                other
            ))),
        }
    }

    pub fn lifecycle_state(&self) -> LifecycleState {
        self.controller.state()
    }

    pub fn suspend(&mut self) -> Result<(), AngNativeError> {
        self.assert_owner_thread();
        self.controller.suspend()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), AngNativeError> {
        self.assert_owner_thread();
        self.controller.resume()?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), AngNativeError> {
        self.assert_owner_thread();
        match self.controller.state() {
            LifecycleState::Destroyed | LifecycleState::Failed => Ok(()),
            LifecycleState::Created => {
                self.controller.initialize()?;
                self.controller.run()?;
                self.controller.stop()?;
                Ok(())
            }
            LifecycleState::Initializing => Err(AngNativeError::Lifecycle(
                "cannot stop while Initializing".to_string(),
            )),
            LifecycleState::Ready => {
                self.controller.run()?;
                self.controller.stop().map_err(Into::into)
            }
            LifecycleState::Stopping => Ok(()),
            LifecycleState::Running | LifecycleState::Suspended => {
                self.controller.stop()?;
                Ok(())
            }
        }
    }

    pub fn set_error_callback(
        &mut self,
        callback: Option<AngNativeErrorCallback>,
        user_data: *mut c_void,
    ) {
        self.error_callback = callback;
        self.error_user_data = user_data;
        let owner = self.owner_id;
        JS_ERROR_STATE.with(|state| {
            if let Some(state) = state.borrow_mut().get_mut(&owner) {
                state.callback = callback.map(|cb| (cb, user_data));
            }
        });
    }

    fn invoke_error_callback(&self, info: &RuntimeErrorInfo) {
        if let Some(callback) = self.error_callback {
            let message = CString::new(info.message.clone()).unwrap_or_default();
            let stack = CString::new(info.stack.clone().unwrap_or_default()).unwrap_or_default();
            unsafe { callback(message.as_ptr(), stack.as_ptr(), self.error_user_data) };
        }
    }

    fn report_error(&mut self, phase: &str, error: &AngNativeError, stack: Option<String>) {
        let info = RuntimeErrorInfo::new(error, phase, stack);
        self.metrics.record_error();
        self.last_error = Some(info.clone());
        self.invoke_error_callback(&info);
    }

    fn drain_js_reported_error(&mut self) {
        let owner = self.owner_id;
        let reported = JS_ERROR_STATE.with(|state| {
            state
                .borrow_mut()
                .get_mut(&owner)
                .and_then(|state| state.last.take())
        });
        if let Some((message, stack)) = reported {
            let error = AngNativeError::ScriptError(message);
            self.report_error("angular_error_handler", &error, stack);
        }
    }

    pub fn metrics(&self) -> &RuntimeMetrics {
        &self.metrics
    }

    pub fn metrics_mut(&mut self) -> &mut RuntimeMetrics {
        &mut self.metrics
    }

    pub fn last_error(&self) -> Option<&RuntimeErrorInfo> {
        self.last_error.as_ref()
    }

    pub fn refresh_metrics(&mut self) {
        self.metrics.refresh_heap(self.runtime.engine().memory_usage());
    }

    pub fn metrics_json(&mut self) -> String {
        self.refresh_metrics();
        self.metrics.to_json()
    }

    pub fn last_error_json(&mut self) -> String {
        self.drain_js_reported_error();
        self.last_error
            .as_ref()
            .map(|e| e.to_json())
            .unwrap_or_else(|| r#"{"code":0,"category":"ok","phase":"none","message":"ok","stack":null}"#.to_string())
    }

    pub fn dispatch_event(
        &mut self,
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> Result<u32, AngNativeError> {
        self.assert_owner_thread();
        self.drain_js_reported_error();
        self.require_running()?;
        let started = Instant::now();
        let result = self.runtime.dispatch_event(node_id, event_name, payload_json);
        self.metrics.record_event(started);
        match result {
            Ok(invoked) => Ok(invoked),
            Err(error) => {
                let error = AngNativeError::from(error);
                self.report_error("dispatch_event", &error, None);
                Err(error)
            }
        }
    }

    pub fn runtime_mut(&mut self) -> &mut AngNativeRuntime<E> {
        self.assert_owner_thread();
        &mut *self.runtime
    }

    pub fn load_script(&mut self, source: &str) -> Result<JsValue, AngNativeError> {
        self.assert_owner_thread();
        self.require_running()?;
        let started = Instant::now();
        let result = self.runtime.load_script(source);
        self.metrics.record_script_load(started);
        self.drain_js_reported_error();
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let error = AngNativeError::from(error);
                self.report_error("load_script", &error, None);
                Err(error)
            }
        }
    }

    pub fn host_mut(&mut self) -> &mut Host {
        &mut *self.host
    }

    pub fn host(&self) -> &Host {
        &*self.host
    }
}

impl<E: JsEngine> Drop for RuntimeBridge<E> {
    fn drop(&mut self) {
        let owner = self.owner_id;
        // Remove only this bridge's slots: another live bridge on the same
        // thread must keep working when this one is destroyed (P1-1).
        CURRENT_BRIDGE_RUNTIME.with(|rt| {
            rt.borrow_mut().remove(&owner);
        });
        CURRENT_BRIDGE_HOST.with(|h| {
            h.borrow_mut().remove(&owner);
        });
        JS_ERROR_STATE.with(|state| {
            state.borrow_mut().remove(&owner);
        });
        clear_js_runtime_dispatch_for_thread(owner);
    }
}

/// Single canonical JS dispatch callback installed on Host.
extern "C" fn js_runtime_dispatch_callback(
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
    owner: usize,
) -> u32 {
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
        // Copy the pointer and release the borrow before the foreign-facing
        // dispatch call (P2-4); route strictly to the originating bridge.
        let ptr = CURRENT_BRIDGE_RUNTIME.with(|rt| rt.borrow().get(&owner).copied());
        match ptr {
            Some(ptr) => {
                let runtime = unsafe { &mut *ptr };
                match runtime.dispatch_event_erased(node_id, event_name_str, payload_str) {
                    Ok(invoked) => invoked,
                    Err(_) => 0,
                }
            }
            None => 0,
        }
    }));
    result.unwrap_or(DISPATCH_PANIC)
}

/// Heap record behind a validated u64 handle. Never exposed across FFI.
struct BridgeRecord {
    bridge: Box<RuntimeBridge>,
}

#[derive(Clone, Copy)]
struct BridgeEntry {
    generation: u64,
    owner_thread: ThreadId,
    /// Address of the heap BridgeRecord. Stored as usize so the registry is Send.
    record: usize,
}

struct BridgeRegistry {
    next_generation: AtomicU64,
    entries: Mutex<HashMap<u64, BridgeEntry>>,
}

unsafe impl Sync for BridgeRegistry {}

static BRIDGES: OnceLock<BridgeRegistry> = OnceLock::new();

fn bridge_registry() -> &'static BridgeRegistry {
    BRIDGES.get_or_init(|| BridgeRegistry {
        next_generation: AtomicU64::new(1),
        entries: Mutex::new(HashMap::new()),
    })
}

/// Recover the registry even after a panic poisoned it. Dropping a poisoned
/// lock otherwise leaks both the BridgeRecord and its registry entry (P3).
fn lock_entries() -> MutexGuard<'static, HashMap<u64, BridgeEntry>> {
    bridge_registry()
        .entries
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lookup_entry(handle: u64) -> Result<BridgeEntry, AngNativeError> {
    let entries = lock_entries();
    entries
        .get(&handle)
        .copied()
        .filter(|entry| entry.generation == handle)
        .ok_or_else(|| AngNativeError::InvalidHandle(format!("handle {handle} is invalid or stale")))
}

thread_local! {
    /// Handles already borrowed via `&mut` on this thread. A native callback
    /// that re-enters the C ABI with the same handle must not create a second
    /// `&mut RuntimeBridge` (P2-4).
    static ACTIVE_BRIDGE_HANDLES: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
}

struct ActiveBridgeGuard(u64);

impl Drop for ActiveBridgeGuard {
    fn drop(&mut self) {
        ACTIVE_BRIDGE_HANDLES.with(|active| {
            active.borrow_mut().remove(&self.0);
        });
    }
}

fn with_bridge<T>(
    handle: u64,
    f: impl FnOnce(&mut RuntimeBridge) -> T,
) -> Result<T, AngNativeError> {
    let entered = ACTIVE_BRIDGE_HANDLES.with(|active| active.borrow_mut().insert(handle));
    if !entered {
        return Err(AngNativeError::Lifecycle(format!(
            "re-entrant access to bridge handle {handle} rejected"
        )));
    }
    let _guard = ActiveBridgeGuard(handle);

    let entry = lookup_entry(handle)?;
    if entry.owner_thread != thread::current().id() {
        return Err(AngNativeError::Lifecycle(
            "bridge handle used from non-owner thread".to_string(),
        ));
    }
    let record = unsafe { &mut *(entry.record as *mut BridgeRecord) };
    Ok(f(&mut record.bridge))
}

/// Returns Ok(true) when the handle is live and owned by this thread,
/// Ok(false) for a live handle owned by another thread, and Err for stale/invalid.
fn bridge_thread_state(handle: u64) -> Result<bool, AngNativeError> {
    let entry = lookup_entry(handle)?;
    Ok(entry.owner_thread == thread::current().id())
}

#[cfg(test)]
fn note_wrong_thread_rejection() {
    WRONG_THREAD_FFI_REJECTIONS.fetch_add(1, AtomicOrdering::SeqCst);
}

#[cfg(not(test))]
fn note_wrong_thread_rejection() {}

/// Shared wrong-thread guard. Returns the sentinel to return from the caller
/// when the call must be rejected.
fn wrong_thread_sentinel(handle: u64, sentinel: u32) -> Option<u32> {
    match bridge_thread_state(handle) {
        Ok(true) => None,
        Ok(false) => {
            note_wrong_thread_rejection();
            Some(sentinel)
        }
        Err(_) => Some(0),
    }
}

fn c_string(value: &str) -> CString {
    CString::new(value).unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_create() -> u64 {
    let result = std::panic::catch_unwind(|| {
        let bridge = RuntimeBridge::new().ok()?;
        let generation = bridge_registry()
            .next_generation
            .fetch_add(1, AtomicOrdering::SeqCst);
        let record = Box::into_raw(Box::new(BridgeRecord { bridge: Box::new(bridge) }));
        let entry = BridgeEntry {
            generation,
            owner_thread: thread::current().id(),
            record: record as usize,
        };
        lock_entries().insert(generation, entry);
        Some(generation)
    });
    result.unwrap_or(Some(0)).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_destroy(handle: u64) {
    if handle == 0 {
        return;
    }
    let entry = match lookup_entry(handle) {
        Ok(entry) => entry,
        Err(_) => return,
    };
    if entry.owner_thread != thread::current().id() {
        note_wrong_thread_rejection();
        return;
    }
    // Destroying while this handle is borrowed would free the bridge that is
    // still on the stack (P2-4).
    let entered = ACTIVE_BRIDGE_HANDLES.with(|active| active.borrow_mut().insert(handle));
    if !entered {
        return;
    }
    let _guard = ActiveBridgeGuard(handle);
    let _ = std::panic::catch_unwind(|| {
        let record = Box::from_raw(entry.record as *mut BridgeRecord);
        lock_entries().remove(&handle);
        drop(record);
    });
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_dispatch_event(
    handle: u64,
    node_id: u64,
    event_name: *const c_char,
    payload_json: *const c_char,
) -> u32 {
    if handle == 0 || event_name.is_null() {
        return 0;
    }
    if let Some(sentinel) = wrong_thread_sentinel(handle, DISPATCH_PANIC) {
        return sentinel;
    }
    let name = match CStr::from_ptr(event_name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let payload = if payload_json.is_null() {
        "{}"
    } else {
        match CStr::from_ptr(payload_json).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_bridge(handle, |bridge| bridge.dispatch_event(node_id, name, payload).unwrap_or(0))
    }));
    result.unwrap_or(Ok(DISPATCH_PANIC)).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_load_script(
    handle: u64,
    source: *const c_char,
) -> i32 {
    if handle == 0 || source.is_null() {
        return 1;
    }
    if let Some(sentinel) = wrong_thread_sentinel(handle, LOAD_SCRIPT_PANIC as u32) {
        return sentinel as i32;
    }
    let source = match CStr::from_ptr(source).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_bridge(handle, |bridge| bridge.load_script(source).is_ok())
    }));
    match result {
        Ok(Ok(true)) => 0,
        Ok(Ok(false) | Err(_)) => 1,
        Err(_) => LOAD_SCRIPT_PANIC,
    }
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_get_host(handle: u64) -> *mut Host {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    if wrong_thread_sentinel(handle, 0).is_some() {
        return std::ptr::null_mut();
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_bridge(handle, |bridge| bridge.host_mut() as *mut Host)
    }));
    result.unwrap_or(Ok(std::ptr::null_mut())).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_lifecycle_state(handle: u64) -> i32 {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return -1;
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| lifecycle_code(bridge.lifecycle_state()))
    });
    result.unwrap_or(Ok(-1)).unwrap_or(-1)
}

fn lifecycle_code(state: LifecycleState) -> i32 {
    match state {
        LifecycleState::Created => 0,
        LifecycleState::Initializing => 1,
        LifecycleState::Ready => 2,
        LifecycleState::Running => 3,
        LifecycleState::Suspended => 4,
        LifecycleState::Stopping => 5,
        LifecycleState::Destroyed => 6,
        LifecycleState::Failed => 7,
    }
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_suspend(handle: u64) -> i32 {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return ErrorCode::InvalidHandle.as_i32();
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| match bridge.suspend() {
            Ok(()) => 0,
            Err(e) => e.ffi_code().as_i32(),
        })
    });
    result.unwrap_or(Ok(ErrorCode::Panic.as_i32())).unwrap_or(ErrorCode::Panic.as_i32())
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_resume(handle: u64) -> i32 {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return ErrorCode::InvalidHandle.as_i32();
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| match bridge.resume() {
            Ok(()) => 0,
            Err(e) => e.ffi_code().as_i32(),
        })
    });
    result.unwrap_or(Ok(ErrorCode::Panic.as_i32())).unwrap_or(ErrorCode::Panic.as_i32())
}

#[no_mangle]
pub extern "C" fn angnative_runtime_bridge_stop(handle: u64) -> i32 {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return ErrorCode::InvalidHandle.as_i32();
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| match bridge.stop() {
            Ok(()) => 0,
            Err(e) => e.ffi_code().as_i32(),
        })
    });
    result.unwrap_or(Ok(ErrorCode::Panic.as_i32())).unwrap_or(ErrorCode::Panic.as_i32())
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_metrics_json(handle: u64) -> *mut c_char {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return std::ptr::null_mut();
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| c_string(&bridge.metrics_json()).into_raw() as *mut c_char)
    });
    result.unwrap_or(Ok(std::ptr::null_mut())).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_last_error_json(handle: u64) -> *mut c_char {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return std::ptr::null_mut();
    }
    let result = std::panic::catch_unwind(|| {
        with_bridge(handle, |bridge| c_string(&bridge.last_error_json()).into_raw() as *mut c_char)
    });
    result.unwrap_or(Ok(std::ptr::null_mut())).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[no_mangle]
pub unsafe extern "C" fn angnative_runtime_bridge_set_error_callback(
    handle: u64,
    callback: Option<AngNativeErrorCallback>,
    user_data: *mut c_void,
) {
    if handle == 0 || !matches!(bridge_thread_state(handle), Ok(true)) {
        return;
    }
    let _ = std::panic::catch_unwind(|| {
        let _ = with_bridge(handle, |bridge| bridge.set_error_callback(callback, user_data));
    });
}

pub fn create_runtime_bridge() -> Result<RuntimeBridge, AngNativeError> {
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

    /// Flagship presentation regression: the real AOT bundle must produce
    /// distinct, non-overlapping layout frames for the Angular counter row.
    /// Requires the generated bundle:
    ///   pnpm compile:aot && pnpm bundle:quickjs
    #[test]
    #[ignore = "requires generated dist/aot/quickjs-bundle.js"]
    fn angular_counter_layout_has_non_overlapping_frames() {
        use angnative_host_core::{NodeSnapshot, TreeSnapshot};
        use angnative_host_ffi::bridge::{Host, LayoutNodeInputJson, LayoutRequestJson};

        fn node_id_for(
            snapshot: &TreeSnapshot,
            predicate: impl Fn(&NodeSnapshot) -> bool,
        ) -> u64 {
            snapshot
                .nodes
                .iter()
                .find(|node| predicate(node))
                .unwrap_or_else(|| panic!("expected native node for predicate"))
                .id
        }

        fn button_id_for(snapshot: &TreeSnapshot, text_id: u64) -> u64 {
            let text_node = snapshot
                .nodes
                .iter()
                .find(|node| node.id == text_id)
                .expect("button text node");
            // The demo binds `[text]` directly on na-button, so the label may
            // live on the button node itself rather than in a child text node.
            if text_node.tag == "na-button" {
                return text_id;
            }
            let parent_id = text_node.parent_id.expect("button text must have a parent");
            let parent = snapshot
                .nodes
                .iter()
                .find(|node| node.id == parent_id)
                .expect("button parent node");
            assert_eq!(parent.tag, "na-button", "button label must be wrapped by na-button");
            parent_id
        }

        fn rect_of(rects: &[serde_json::Value], node_id: u64) -> (f32, f32, f32, f32) {
            let rect = rects
                .iter()
                .find(|rect| rect["node_id"].as_u64() == Some(node_id))
                .unwrap_or_else(|| panic!("no layout rect for node {node_id}"));
            (
                rect["x"].as_f64().expect("x") as f32,
                rect["y"].as_f64().expect("y") as f32,
                rect["width"].as_f64().expect("width") as f32,
                rect["height"].as_f64().expect("height") as f32,
            )
        }

        /// Layout frames are parent-relative; sum ancestors for a screen-space
        /// rectangle before checking overlap.
        fn absolute_rect_for(
            snapshot: &TreeSnapshot,
            rects: &[serde_json::Value],
            node_id: u64,
        ) -> (f32, f32, f32, f32) {
            let mut x = 0.0;
            let mut y = 0.0;
            let mut current = Some(node_id);
            while let Some(id) = current {
                let (dx, dy, _, _) = rect_of(rects, id);
                x += dx;
                y += dy;
                current = snapshot
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
                    .and_then(|node| node.parent_id);
            }
            let (_, _, width, height) = rect_of(rects, node_id);
            (x, y, width, height)
        }

        fn overlaps(a: &(f32, f32, f32, f32), b: &(f32, f32, f32, f32)) -> bool {
            a.0 < b.0 + b.2
                && b.0 < a.0 + a.2
                && a.1 < b.1 + b.3
                && b.1 < a.1 + a.3
        }

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/quickjs-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot && pnpm bundle:quickjs` first");

        let mut bridge = create_runtime_bridge().unwrap();
        bridge.load_script(&source).expect("bundle must parse and run");

        let snapshot = bridge.host_mut().engine().snapshot();
        assert!(
            snapshot.nodes.iter().any(|node| node.id == 1),
            "Angular root node 1 missing from host snapshot"
        );

        let inputs = snapshot
            .nodes
            .iter()
            .map(|node| LayoutNodeInputJson {
                node_id: node.id,
                styles: node.styles.clone(),
                children: node.children.clone(),
            })
            .collect();
        let request = LayoutRequestJson {
            root_id: 1,
            nodes: inputs,
            available_width: 411.0,
            available_height: 800.0,
        };
        let request_json = serde_json::to_string(&request).expect("request must serialize");

        let mut host = Host::new();
        let response_json = host
            .run_layout_json(&request_json)
            .expect("layout must succeed with canonicalized styles");
        let response: serde_json::Value =
            serde_json::from_str(&response_json).expect("layout response must be JSON");
        assert!(
            response.get("error").is_none(),
            "layout returned an error: {response_json}"
        );
        let rects = response
            .get("nodes")
            .and_then(|nodes| nodes.as_array())
            .expect("layout response must contain nodes");

        let count_text_id = node_id_for(&snapshot, |node| {
            node.text_content.as_deref() == Some("Count: 0")
        });
        let minus_text_id = node_id_for(&snapshot, |node| {
            matches!(node.text_content.as_deref(), Some("−") | Some("-"))
        });
        let reset_text_id = node_id_for(&snapshot, |node| {
            node.text_content.as_deref() == Some("reset")
        });
        let plus_text_id = node_id_for(&snapshot, |node| {
            node.text_content.as_deref() == Some("+")
        });

        let count = absolute_rect_for(&snapshot, rects, count_text_id);
        let minus = absolute_rect_for(&snapshot, rects, button_id_for(&snapshot, minus_text_id));
        let reset = absolute_rect_for(&snapshot, rects, button_id_for(&snapshot, reset_text_id));
        let plus = absolute_rect_for(&snapshot, rects, button_id_for(&snapshot, plus_text_id));

        let controls = [
            ("Count: 0", count),
            ("minus", minus),
            ("reset", reset),
            ("plus", plus),
        ];
        for (label, rect) in &controls {
            eprintln!("{label}: ({:.0}, {:.0}, {:.0}, {:.0})", rect.0, rect.1, rect.2, rect.3);
        }
        for (label, rect) in &controls {
            assert!(
                rect.2 > 0.0 && rect.3 > 0.0,
                "{label} has zero layout size: {rect:?}"
            );
        }
        for i in 0..controls.len() {
            for j in (i + 1)..controls.len() {
                assert!(
                    !overlaps(&controls[i].1, &controls[j].1),
                    "{} overlaps {}: {:?} / {:?}",
                    controls[i].0,
                    controls[j].0,
                    controls[i].1,
                    controls[j].1
                );
            }
        }
        assert!(
            minus.0 < reset.0 && reset.0 < plus.0,
            "button row lost left-to-right order: minus={minus:?} reset={reset:?} plus={plus:?}"
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

    static ERROR_CALLBACK_HITS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    extern "C" fn test_error_callback(
        _message: *const c_char,
        _stack: *const c_char,
        _user_data: *mut c_void,
    ) {
        ERROR_CALLBACK_HITS.fetch_add(1, AtomicOrdering::SeqCst);
    }

    #[test]
    fn test_thread_affinity_and_validated_handles() {
        use std::ffi::CString;

        let handle = angnative_runtime_bridge_create();
        assert_ne!(handle, 0, "bridge creation must succeed");
        assert_ne!(angnative_runtime_bridge_lifecycle_state(handle), -1);

        let before = WRONG_THREAD_FFI_REJECTIONS.load(AtomicOrdering::SeqCst);
        let result = thread::spawn(move || {
            let name = CString::new("press").unwrap();
            unsafe { angnative_runtime_bridge_dispatch_event(handle, 1, name.as_ptr(), CString::new("{}").unwrap().as_ptr()) }
        })
        .join()
        .expect("wrong-thread C ABI dispatch must not panic");
        assert_eq!(result, DISPATCH_PANIC);
        assert_eq!(WRONG_THREAD_FFI_REJECTIONS.load(AtomicOrdering::SeqCst), before + 1);

        unsafe { angnative_runtime_bridge_destroy(handle) };

        // Generation-checked stale handle: no pointer dereference, no panic.
        let name = CString::new("press").unwrap();
        assert_eq!(
            unsafe { angnative_runtime_bridge_dispatch_event(handle, 1, name.as_ptr(), std::ptr::null()) },
            0
        );
        assert!(unsafe { angnative_runtime_bridge_get_host(handle) }.is_null());
        assert_eq!(angnative_runtime_bridge_lifecycle_state(handle), -1);
        assert_eq!(unsafe { angnative_runtime_bridge_dispatch_event(u64::MAX, 1, name.as_ptr(), std::ptr::null()) }, 0);
    }

    #[test]
    fn test_lifecycle_transitions_are_deterministic() {
        let mut bridge = create_runtime_bridge().unwrap();
        assert_eq!(bridge.lifecycle_state(), LifecycleState::Running);

        bridge.suspend().unwrap();
        assert_eq!(bridge.lifecycle_state(), LifecycleState::Suspended);
        let error = bridge.dispatch_event(1, "press", "{}").unwrap_err();
        assert_eq!(error.ffi_code(), ErrorCode::Lifecycle);

        bridge.resume().unwrap();
        assert_eq!(bridge.lifecycle_state(), LifecycleState::Running);
        assert_eq!(bridge.dispatch_event(1, "press", "{}").unwrap(), 0);

        bridge.stop().unwrap();
        assert_eq!(bridge.lifecycle_state(), LifecycleState::Destroyed);
        assert_eq!(bridge.dispatch_event(1, "press", "{}").unwrap_err().ffi_code(), ErrorCode::Lifecycle);
    }

    #[test]
    fn test_metrics_error_handler_and_native_callback() {
        let handle = angnative_runtime_bridge_create();
        assert_ne!(handle, 0);
        ERROR_CALLBACK_HITS.store(0, AtomicOrdering::SeqCst);
        unsafe {
            angnative_runtime_bridge_set_error_callback(handle, Some(test_error_callback), std::ptr::null_mut());
        }

        let script = CString::new("na_reportError('boom from Angular', 'stack-line-1'); 0;").unwrap();
        assert_eq!(unsafe { angnative_runtime_bridge_load_script(handle, script.as_ptr()) }, 0);

        let raw = unsafe { angnative_runtime_bridge_last_error_json(handle) };
        assert!(!raw.is_null());
        let json = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
        unsafe { angnative_runtime_bridge_free_string(raw) };
        assert!(json.contains("boom from Angular"), "last error JSON: {json}");
        assert!(json.contains("angular_error_handler"), "last error JSON: {json}");
        assert_eq!(
            ERROR_CALLBACK_HITS.load(AtomicOrdering::SeqCst),
            1,
            "na_reportError must notify the native callback exactly once"
        );

        let raw = unsafe { angnative_runtime_bridge_metrics_json(handle) };
        assert!(!raw.is_null());
        let json = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
        unsafe { angnative_runtime_bridge_free_string(raw) };
        assert!(json.contains("\"script_load_count\":1"), "metrics JSON: {json}");
        assert!(json.contains("\"reported_error_count\":1"), "metrics JSON: {json}");
        assert!(json.contains("QuickJS"), "metrics JSON: {json}");

        unsafe { angnative_runtime_bridge_destroy(handle) };
    }

    /// P1-1 regression: two live bridges on one thread must route each Host's
    /// event to its own runtime, and destroying one must not disable the other.
    #[test]
    fn two_bridges_on_one_thread_do_not_cross_route() {
        let mut a = create_runtime_bridge().unwrap();
        let mut b = create_runtime_bridge().unwrap();

        for bridge in [&mut a, &mut b] {
            bridge
                .load_script(
                    "globalThis.hits = 0; \
                     globalThis.__angnativeDispatchListener = function () { hits += 1; }; \
                     na_addEventListener(1, 'click', 7); 0;",
                )
                .unwrap();
        }

        let host_a = a.host() as *const Host as *mut Host;
        let host_b = b.host() as *const Host as *mut Host;

        assert_eq!(unsafe { (*host_a).dispatch_event(1, "click", "{}") }, 1);
        assert_eq!(a.runtime_mut().eval("hits").unwrap(), JsValue::Number(1.0));
        assert_eq!(b.runtime_mut().eval("hits").unwrap(), JsValue::Number(0.0));

        assert_eq!(unsafe { (*host_b).dispatch_event(1, "click", "{}") }, 1);
        assert_eq!(a.runtime_mut().eval("hits").unwrap(), JsValue::Number(1.0));
        assert_eq!(b.runtime_mut().eval("hits").unwrap(), JsValue::Number(1.0));

        drop(b);
        assert_eq!(unsafe { (*host_a).dispatch_event(1, "click", "{}") }, 1);
        assert_eq!(a.runtime_mut().eval("hits").unwrap(), JsValue::Number(2.0));
    }

    /// P2-4 regression: a native callback invoked from inside `with_bridge`
    /// must not obtain a second `&mut RuntimeBridge` for the same handle.
    #[test]
    fn native_callback_cannot_reenter_the_same_bridge_handle() {
        use std::sync::atomic::AtomicI32;

        static HANDLE: AtomicU64 = AtomicU64::new(0);
        static REENTERED_STATE: AtomicI32 = AtomicI32::new(99);

        extern "C" fn reenter(
            _message: *const c_char,
            _stack: *const c_char,
            _user_data: *mut c_void,
        ) {
            let handle = HANDLE.load(AtomicOrdering::SeqCst);
            REENTERED_STATE.store(
                angnative_runtime_bridge_lifecycle_state(handle),
                AtomicOrdering::SeqCst,
            );
        }

        let handle = angnative_runtime_bridge_create();
        assert_ne!(handle, 0);
        HANDLE.store(handle, AtomicOrdering::SeqCst);
        unsafe {
            angnative_runtime_bridge_set_error_callback(handle, Some(reenter), std::ptr::null_mut())
        };

        let script = CString::new("na_reportError('reentrant probe', ''); 0;").unwrap();
        assert_eq!(
            unsafe { angnative_runtime_bridge_load_script(handle, script.as_ptr()) },
            0
        );
        assert_eq!(
            REENTERED_STATE.load(AtomicOrdering::SeqCst),
            -1,
            "re-entrant FFI access must be rejected instead of aliasing &mut"
        );
        assert_eq!(angnative_runtime_bridge_lifecycle_state(handle), 3);

        unsafe { angnative_runtime_bridge_destroy(handle) };
    }

    #[test]
    fn test_reference_engine_proves_bridge_engine_abstraction() {
        let mut bridge = RuntimeBridge::with_engine(
            angnative_host_engine::ReferenceEngine::with_name("GateFReference"),
        )
        .unwrap();
        assert_eq!(bridge.metrics().engine_name, "GateFReference");
        assert_eq!(bridge.lifecycle_state(), LifecycleState::Running);
        assert_eq!(bridge.load_script("var x = 1;").unwrap(), JsValue::Null);
        assert_eq!(bridge.metrics().script_load_count, 1);
        assert_eq!(bridge.runtime_mut().engine().engine_name(), "GateFReference");
    }

    /// P1-09: with a real AOT bundle loaded, the Angular ErrorHandler host
    /// hook (`__angnativeOnError` → `na_reportError`) must reach the structured
    /// runtime error/metrics boundary. The throw is injected after the bundle
    /// because generated demo components do not deliberately throw.
    #[test]
    #[ignore = "requires generated dist/aot/quickjs-bundle.js"]
    fn angular_error_handler_receives_aot_runtime_throw() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dist/aot/quickjs-bundle.js"
        );
        let source = std::fs::read_to_string(path)
            .expect("run `pnpm compile:aot && pnpm bundle:quickjs` first");

        let handle = angnative_runtime_bridge_create();
        assert_ne!(handle, 0);
        assert_eq!(unsafe { angnative_runtime_bridge_load_script(handle, CString::new(source).unwrap().as_ptr()) }, 0);

        let trigger = CString::new(
            "globalThis.__angnativeOnError(new Error('angular error handler smoke'), 'stack-line-42'); 0;",
        )
        .unwrap();
        assert_eq!(unsafe { angnative_runtime_bridge_load_script(handle, trigger.as_ptr()) }, 0);

        let raw = unsafe { angnative_runtime_bridge_last_error_json(handle) };
        assert!(!raw.is_null());
        let json = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
        unsafe { angnative_runtime_bridge_free_string(raw) };
        assert!(json.contains("angular error handler smoke"), "last error JSON: {json}");
        assert!(json.contains("angular_error_handler"), "last error JSON: {json}");
        assert!(json.contains("stack-line-42"), "last error JSON: {json}");

        let raw = unsafe { angnative_runtime_bridge_metrics_json(handle) };
        assert!(!raw.is_null());
        let metrics = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
        unsafe { angnative_runtime_bridge_free_string(raw) };
        assert!(metrics.contains("\"reported_error_count\":1"), "metrics JSON: {metrics}");

        unsafe { angnative_runtime_bridge_destroy(handle) };
    }

    #[test]
    #[ignore = "stress suite: 10k dispatches"]
    fn stress_10000_dispatches_keep_one_delivery_per_event() {
        let mut bridge = create_runtime_bridge().unwrap();
        bridge
            .runtime_mut()
            .eval("var _stress = 0; function na_onEvent() { _stress++; } na_addEventListener(7, 'press', 1);")
            .unwrap();
        for _ in 0..10_000 {
            assert_eq!(bridge.dispatch_event(7, "press", "{}").unwrap(), 1);
        }
        assert_eq!(
            bridge.runtime_mut().eval("_stress").unwrap(),
            JsValue::Number(10_000.0)
        );
        assert_eq!(bridge.metrics().event_count, 10_000);
    }

    #[test]
    #[ignore = "stress suite: 1k bridge create/destroy cycles"]
    fn stress_1000_create_destroy_cycles_do_not_leak_handles() {
        for _ in 0..1_000 {
            let handle = angnative_runtime_bridge_create();
            assert_ne!(handle, 0);
            assert!(lookup_entry(handle).is_ok(), "fresh handle must be registered");
            unsafe { angnative_runtime_bridge_destroy(handle) };
            assert!(lookup_entry(handle).is_err(), "destroyed handle must be gone");
        }
    }

    #[test]
    #[ignore = "memory-growth analysis: registry baseline after 200 cycles"]
    fn memory_growth_analysis_registry_returns_to_baseline() {
        // Assert on this test's own handles, not the global count: the ignored
        // suite runs in parallel, so other tests may legitimately hold bridges.
        let mut created = Vec::new();
        for _ in 0..200 {
            let handle = angnative_runtime_bridge_create();
            assert_ne!(handle, 0);
            assert!(lookup_entry(handle).is_ok(), "fresh handle must be registered");
            created.push(handle);
            unsafe { angnative_runtime_bridge_destroy(handle) };
            assert!(lookup_entry(handle).is_err(), "destroyed handle must be gone");
        }
        assert!(created.iter().all(|h| lookup_entry(*h).is_err()));
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
