/**
 * reference_engine.rs — in-repo second `JsEngine` implementation (Gate F).
 *
 * This is NOT Hermes or V8. It is a deliberately small, deterministic engine
 * used to prove that `RuntimeBridge` and `AngNativeRuntime` depend only on the
 * `JsEngine` trait. It records scripts and dispatches registered functions;
 * it does not parse JavaScript.
 */
use crate::engine::{EngineError, JsEngine, JsValue};
use std::collections::HashMap;
use std::rc::Rc;

type RegisteredFunction = Rc<dyn Fn(Vec<JsValue>) -> Result<JsValue, EngineError>>;

/// Minimal engine used for abstraction/conformance tests.
#[derive(Clone)]
pub struct ReferenceEngine {
    name: &'static str,
    functions: HashMap<String, RegisteredFunction>,
    globals: HashMap<String, JsValue>,
    scripts: Vec<String>,
    gc_calls: u32,
}

impl std::fmt::Debug for ReferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceEngine")
            .field("name", &self.name)
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .field("scripts", &self.scripts.len())
            .field("gc_calls", &self.gc_calls)
            .finish()
    }
}

impl ReferenceEngine {
    pub fn new() -> Self {
        Self {
            name: "ReferenceEngine",
            functions: HashMap::new(),
            globals: HashMap::new(),
            scripts: Vec::new(),
            gc_calls: 0,
        }
    }

    pub fn with_name(name: &'static str) -> Self {
        let mut engine = Self::new();
        engine.name = name;
        engine
    }

    pub fn scripts(&self) -> &[String] {
        &self.scripts
    }

    pub fn gc_calls(&self) -> u32 {
        self.gc_calls
    }
}

impl Default for ReferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl JsEngine for ReferenceEngine {
    fn eval(&mut self, source: &str) -> Result<JsValue, EngineError> {
        self.scripts.push(source.to_string());
        Ok(JsValue::Null)
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

    fn memory_usage(&self) -> Option<usize> {
        Some(0)
    }

    fn engine_name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AngNativeRuntime, RuntimeConfig};

    #[test]
    fn reference_engine_satisfies_runtime_api() {
        let mut runtime = AngNativeRuntime::new(ReferenceEngine::new(), RuntimeConfig::default());
        runtime.install_api().unwrap();
        assert_eq!(runtime.engine().engine_name(), "ReferenceEngine");
        // install_api evaluates the `na` namespace bootstrap once.
        assert_eq!(runtime.engine().scripts().len(), 1);
        // A second, ordinary eval must flow through the same engine API.
        runtime.eval("var gateF = 1;").unwrap();
        assert_eq!(runtime.engine().scripts().len(), 2);
    }

    #[test]
    fn reference_engine_dispatches_registered_functions() {
        let mut engine = ReferenceEngine::new();
        engine.register_function("sum", |args| {
            let total: f64 = args
                .iter()
                .map(|v| match v {
                    JsValue::Number(n) => *n,
                    _ => 0.0,
                })
                .sum();
            Ok(JsValue::Number(total))
        });
        assert_eq!(
            engine
                .call_function("sum", &[JsValue::Number(2.0), JsValue::Number(3.0)])
                .unwrap(),
            JsValue::Number(5.0)
        );
    }
}
