/**
 * runtime_controller.rs — Lightweight lifecycle state machine.
 *
 * Ownership model (fixed 2026-11-22):
 * - This controller does NOT own a QuickJS runtime. The single authoritative
 *   owner is `angnative-runtime-bridge::RuntimeBridge` (Host + QuickJS +
 *   AngNativeRuntime). This type only tracks lifecycle state for shells that
 *   want an explicit Created→Ready→Running→… sequence.
 * - No thread-local runtime storage here (that was a duplicate event/runtime
 *   path and caused P0-01/P0-02 in docs/architecture/issue-register.md).
 * - `resume()` is idempotent.
 */
use crate::RuntimeConfig;
use std::sync::{Arc, Mutex};

/// Application lifecycle states.
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

/// Valid state transitions.
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
            max_heap_size: 10 * 1024 * 1024,
            enable_inspector: false,
        }
    }
}

/// Tracks lifecycle state only — no runtime, no Host, no event dispatch.
/// The actual runtime lives in `RuntimeBridge`.
#[derive(Clone)]
pub struct RuntimeController {
    state: Arc<Mutex<LifecycleState>>,
    config: RuntimeControllerConfig,
    /// Kept for API compatibility; currently unused by this lightweight controller.
    _runtime_config: RuntimeConfig,
}

impl RuntimeController {
    pub fn new(config: RuntimeControllerConfig) -> Result<Self, anyhow::Error> {
        Ok(Self {
            state: Arc::new(Mutex::new(LifecycleState::Created)),
            config,
            _runtime_config: RuntimeConfig::default(),
        })
    }

    fn transition(&self, new_state: LifecycleState) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
        if !VALID_TRANSITIONS.contains(&(*state, new_state)) {
            return Err(anyhow::anyhow!(
                "Invalid state transition: {:?} -> {:?}",
                *state,
                new_state
            ));
        }
        if self.config.debug {
            eprintln!("RuntimeController: {:?} -> {:?}", *state, new_state);
        }
        *state = new_state;
        Ok(())
    }

    pub fn initialize(&self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Initializing)?;
        // Bundle loading is done by RuntimeBridge::runtime_mut().eval(...)
        self.transition(LifecycleState::Ready)
    }

    pub fn run(&self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Running)
    }

    pub fn suspend(&self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Suspended)
    }

    /// Resume is idempotent: calling it while already Running is a no-op (P1 fix).
    pub fn resume(&self) -> Result<(), anyhow::Error> {
        if self.state() == LifecycleState::Running {
            return Ok(());
        }
        self.transition(LifecycleState::Running)
    }

    pub fn stop(&self) -> Result<(), anyhow::Error> {
        self.transition(LifecycleState::Stopping)?;
        self.transition(LifecycleState::Destroyed)
    }

    pub fn state(&self) -> LifecycleState {
        // Recover a poisoned state lock instead of `expect`, which aborts in
        // Android release builds (`panic = "abort"`). The stored state is a
        // plain Copy value, so reading it after a panic is safe (P3).
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
