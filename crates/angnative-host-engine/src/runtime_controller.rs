/**
 * runtime_controller.rs — Application runtime lifecycle controller.
 *
 * Owns the authoritative application runtime: Host, QuickJS engine,
 * Angular runtime, and coordinates lifecycle transitions.
 *
 * Design: RuntimeController owns AngNativeRuntime directly (no raw pointers).
 * Thread-local provides access for C callbacks on the owning thread.
 * Uses RefCell with try_borrow_mut() for reentrancy safety.
 */
use crate::{RuntimeConfig};
use crate::quickjs::QuickJsEngine;
use crate::engine::JsEngine;
use angnative_host_ffi::bridge::Host;
use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::os::raw::c_int;
use std::thread_local;

/// Application lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Created,
    Initializing,
    Ready,
    Running,
    Suspended,
    Stopping,
    Destroyed,
    Failed,
}

/// Valid state transitions
const VALID_TRANSITIONS: &[(LifecycleState, LifecycleState)] = &[
    (LifecycleState::Created, LifecycleState::Initializing),
    (LifecycleState::Initializing, LifecycleState::Ready),
    (LifecycleState::Initializing, LifecycleState::Failed),
    (LifecycleState::Ready, LifecycleState::Running),
    (LifecycleState::Running, LifecycleState::Suspended),
    (LifecycleState::Running, LifecycleState::Stopping),
    (LifecycleState::Suspended, LifecycleState::Running),
    (LifecycleState::Suspended, LifecycleState::Stopping),
    (LifecycleState::Stopping, LifecycleState::Destroyed),
];

// Thread-local storage for the current runtime's Angular runtime pointer
thread_local! {
    static CURRENT_ANGULAR_RUNTIME: RefCell<Option<AngNativeRuntime<QuickJsEngine>>> = RefCell::new(None);
}

// Thread-local storage for the current runtime's microtask queue
thread_local! {
    static MICROTASK_QUEUE: RefCell<Vec<Box<dyn FnOnce() + Send>>> = RefCell::new(Vec::new());
}

/// Application runtime controller - single authoritative instance per app.
pub struct RuntimeController {
    host: Host,
    state: LifecycleState,
    config: RuntimeControllerConfig,
    runtime: AngNativeRuntime<QuickJsEngine>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RuntimeControllerConfig {
    pub debug: bool,
    pub max_heap_size: usize,
    pub enable_inspector: bool,
}

impl Default for RuntimeControllerConfig {
    fn default() -> Self {
        Self {
            debug: false,
            max_heap_size: 10 * 1024 * 1024, // 10MB
            enable_inspector: false,
        }
    }
}

impl RuntimeController {
    /// Create a new runtime controller.
    pub fn new(config: RuntimeControllerConfig) -> Result<Self, anyhow::Error> {
        let host = Host::new();
        let engine = QuickJsEngine::new()?;
        let mut runtime = AngNativeRuntime::new(engine, RuntimeConfig::default());
        
        // Install AngNative API (na.*)
        runtime.install_api()?;
        
        // Install DOM/platform shims
        crate::quickjs_shims::install_dom_shims(&mut runtime)?;
        
        // Install module system
        crate::quickjs_shims::install_module_system(&mut runtime)?;
        
        // Register console functions
        crate::quickjs_shims::register_console_functions(&mut runtime)?;
        
        // Store runtime pointer in thread-local for C callback access
        CURRENT_ANGULAR_RUNTIME.with(|cell| {
            *cell.borrow_mut() = Some(runtime);
        });
        
        Ok(Self {
            host,
            state: LifecycleState::Created,
            config,
            runtime,
        })
    }
    
    /// Transition to a new lifecycle state.
    fn transition(&mut self, new_state: LifecycleState) -> Result<(), anyhow::Error> {
        if !VALID_TRANSITIONS.contains(&(self.state, new_state)) {
            return Err(anyhow::anyhow!("Invalid state transition: {:?} -> {:?}", self.state, new_state));
        }
        
        if self.config.debug {
            eprintln!("RuntimeController: {:?} -> {:?}", self.state, new_state);
        }
        
        self.state = new_state;
        Ok(())
    }
    
    /// Initialize the runtime (load Angular bundle, bootstrap).
    pub fn initialize(&mut self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Initializing)?;
        
        // TODO: Load Angular AOT bundle
        // self.load_application_bundle()?;
        
        self.transition(LifecycleState::Ready)?;
        Ok(())
    }
    
    /// Start the application (bootstrap Angular).
    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Running)?;
        
        // TODO: Bootstrap Angular application
        // self.bootstrap_angular()?;
        
        Ok(())
    }
    
    /// Suspend the application.
    pub fn suspend(&mut self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Suspended)?;
        Ok(())
    }
    
    /// Resume the application.
    pub fn resume(&mut self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Running)?;
        Ok(())
    }
    
    /// Stop the application.
    pub fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Stopping)?;
        
        // Clear microtasks
        Self::drain_microtasks_static();
        
        // Clear thread-local runtime pointer
        if self.runtime_in_thread_local {
            CURRENT_ANGULAR_RUNTIME.with(|cell| {
                *cell.borrow_mut() = None;
            });
            self.runtime_in_thread_local = false;
        }
        
        self.transition(LifecycleState::Destroyed)?;
        Ok(())
    }
    
    /// Get current lifecycle state.
    pub fn state(&self) -> LifecycleState {
        self.state
    }
    
    /// Get mutable access to Host.
    pub fn host_mut(&mut self) -> &mut Host {
        &mut self.host
    }
    
    /// Get immutable access to Host.
    pub fn host(&self) -> &Host {
        &self.host
    }
    
    /// Load Angular AOT bundle into QuickJS.
    pub fn load_application(&mut self, bundle_js: &str) -> Result<(), anyhow::Error> {
        // Load as ES module
        self.runtime.engine().eval(&format!(
            r#"
                (async () => {{
                    const module = {{ exports: {{}} }};
                    const exports = module.exports;
                    {}
                    return module.exports;
                }})()
            "#,
            bundle_js
        ))?;
        Ok(())
    }
    
    /// Bootstrap Angular application.
    pub fn bootstrap_angular(&mut self, bootstrap_js: &str) -> Result<(), anyhow::Error> {
        self.runtime.engine().eval(bootstrap_js)?;
        // Drain any microtasks from bootstrap
        Self::drain_microtasks_static();
        Ok(())
    }
    
    /// Drain microtask queue (static method for callbacks).
    fn drain_microtasks_static() {
        MICROTASK_QUEUE.with(|queue| {
            let mut tasks = queue.borrow_mut();
            while let Some(task) = tasks.pop() {
                task();
            }
        });
    }
    
    /// Drain microtask queue.
    pub fn drain_microtasks(&mut self) {
        Self::drain_microtasks_static();
    }
    
    /// Schedule a microtask.
    pub fn schedule_microtask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        MICROTASK_QUEUE.with(|queue| {
            queue.borrow_mut().push(Box::new(f));
        });
    }
    
    /// Process pending events and microtasks.
    pub fn pump(&mut self) -> Result<(), anyhow::Error> {
        // Drain microtasks
        Self::drain_microtasks_static();
        
        // Run QuickJS GC periodically
        with_current_runtime_unchecked(|runtime| {
            runtime.engine().gc()
        })??;
        
        Ok(())
    }
    
    /// Dispatch a native event to the JS runtime (for C callback).
    /// Uses try_borrow_mut to prevent reentrant calls.
    fn dispatch_event_from_callback(
        node_id: u64,
        event_name: &str,
        payload_json: &str,
    ) -> u32 {
        with_current_runtime_unchecked(|runtime| {
            match runtime.dispatch_event(node_id, event_name, payload_json) {
                Ok(invoked) => invoked,
                Err(_) => 0,
            }
        })
    }
}