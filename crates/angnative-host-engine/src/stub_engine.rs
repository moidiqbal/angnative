/**
 * stub_engine.rs — non-executing `JsEngine` test double / contributor template.
 *
 * This is NOT Hermes, NOT V8, and NOT a second production engine. It exists so
 * a future engine adapter has a minimal skeleton to copy: implement the six
 * trait methods, keep the source string honest, and let the shared conformance
 * suite prove the API shape. It deliberately refuses to evaluate JavaScript;
 * `ReferenceEngine` already covers the "record and return Null" behaviour.
 *
 * Gated behind the default-off `engine-stub` Cargo feature so it never enters
 * the shipping path. No engine name in this module may claim integration,
 * verification or a real third-party engine (guarded by a test below).
 */
use crate::engine::{EngineError, JsEngine, JsValue};
use std::collections::HashMap;
use std::rc::Rc;

/// Exact diagnostic name. Keep the words "non-executing test double" so no
/// reader can mistake this for an embedded third-party engine.
pub const STUB_ENGINE_NAME: &str = "StubEngine (non-executing test double)";

type RegisteredFunction = Rc<dyn Fn(Vec<JsValue>) -> Result<JsValue, EngineError>>;

/// Minimal, deterministic `JsEngine` template. Does not parse or run JS.
#[derive(Clone)]
pub struct StubEngine {
    functions: HashMap<String, RegisteredFunction>,
    globals: HashMap<String, JsValue>,
    gc_calls: u32,
}

impl std::fmt::Debug for StubEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StubEngine")
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .field("globals", &self.globals.keys().collect::<Vec<_>>())
            .field("gc_calls", &self.gc_calls)
            .finish()
    }
}

impl Default for StubEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StubEngine {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            globals: HashMap::new(),
            gc_calls: 0,
        }
    }

    pub fn gc_calls(&self) -> u32 {
        self.gc_calls
    }
}

impl JsEngine for StubEngine {
    fn eval(&mut self, _source: &str) -> Result<JsValue, EngineError> {
        Err(EngineError::Internal(
            "StubEngine eval is intentionally unimplemented (non-executing test double)".to_string(),
        ))
    }

    fn call_function(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue, EngineError> {
        let function = self.functions.get(name).cloned().ok_or_else(|| {
            EngineError::CallError(format!("function '{name}' is not registered"))
        })?;
        function(args.to_vec())
    }

    fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(Vec<JsValue>) -> Result<JsValue, EngineError> + 'static,
        Self: Sized,
    {
        self.functions.insert(name.to_string(), Rc::new(func));
    }

    fn set_global(&mut self, name: &str, value: JsValue) -> Result<(), EngineError> {
        self.globals.insert(name.to_string(), value);
        Ok(())
    }

    fn get_global(&self, name: &str) -> Option<JsValue> {
        self.globals.get(name).cloned()
    }

    fn gc(&mut self) -> Result<(), EngineError> {
        self.gc_calls += 1;
        Ok(())
    }

    fn drain_microtasks(&mut self) -> Result<u32, EngineError> {
        // No JS realm exists here, so there is no queue to drain. Override the
        // trait default (which uses `eval`) so this double never pretends to
        // execute a script while satisfying Tier A conformance.
        Ok(0)
    }

    fn memory_usage(&self) -> Option<usize> {
        None
    }

    fn engine_name(&self) -> &'static str {
        STUB_ENGINE_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_engine_name_cannot_claim_a_real_engine_or_integration() {
        let name = StubEngine::new().engine_name();
        assert_eq!(name, STUB_ENGINE_NAME);
        for forbidden in ["Hermes", "hermes", "V8", "verified", "integrated"] {
            assert!(
                !name.contains(forbidden),
                "stub engine name must not claim {forbidden}"
            );
        }
    }

    #[test]
    fn stub_eval_is_explicitly_unimplemented() {
        let err = StubEngine::new().eval("1 + 2").unwrap_err();
        match err {
            EngineError::Internal(message) => {
                assert!(message.to_ascii_lowercase().contains("stub"));
                assert!(message.to_ascii_lowercase().contains("unimplemented"));
            }
            other => panic!("expected Internal for stub eval, got {other:?}"),
        }
    }
}
