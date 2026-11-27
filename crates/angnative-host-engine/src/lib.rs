/**
 * lib.rs — angnative-host-engine: engine-neutral JS/TS runtime.
 *
 * Identity D-5: engine-neutral trait; QuickJS-ng (rquickjs) default;
 * JavaScriptCore on Apple where available. The TS side never learns which
 * engine is embedded (spec D5).
 *
 * Modules:
 *   - engine       — `JsEngine` trait + `JsValue` + `EngineError`
 *   - quickjs      — QuickJS-ng (rquickjs) implementation
 *   - runtime      — AngNative JS runtime API exposed to scripts
 *   - quickjs_shims — DOM/platform shims for QuickJS
 *   - runtime_controller — Application lifecycle controller
 *   - errors — workspace error boundary + runtime metrics (P1-11 / P3-04)
 *   - reference_engine — second in-repo JsEngine for abstraction tests (Gate F)
 */
pub mod engine;
pub mod errors;
pub mod quickjs;
pub mod reference_engine;
#[cfg(feature = "engine-stub")]
pub mod stub_engine;
pub mod runtime;
pub mod quickjs_shims;
pub mod runtime_controller;

pub use engine::{EngineError, JsEngine, JsValue};
pub use errors::{AngNativeError, ErrorCode, RuntimeErrorInfo, RuntimeMetrics};
pub use reference_engine::ReferenceEngine;
pub use runtime::{AngNativeRuntime, RuntimeConfig};
pub use runtime_controller::{RuntimeController, RuntimeControllerConfig, LifecycleState};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_runtime_controller_integration() {
        let controller = RuntimeController::new(RuntimeControllerConfig::default()).unwrap();
        assert_eq!(controller.state(), LifecycleState::Created);
        
        controller.initialize().unwrap();
        assert_eq!(controller.state(), LifecycleState::Ready);
        
        controller.run().unwrap();
        assert_eq!(controller.state(), LifecycleState::Running);
        
        controller.stop().unwrap();
        assert_eq!(controller.state(), LifecycleState::Destroyed);
    }
}
