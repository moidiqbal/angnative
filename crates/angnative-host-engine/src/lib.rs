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
 */
pub mod engine;
pub mod quickjs;
pub mod runtime;

pub use engine::{EngineError, JsEngine, JsValue};
pub use runtime::{AngNativeRuntime, RuntimeConfig};
