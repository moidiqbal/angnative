/**
 * engine.rs — Engine-neutral JS runtime trait.
 *
 * Defines the `JsEngine` trait that platform shells depend on. The TS side
 * never learns which engine is embedded — it only sends commands through the
 * command protocol (host-core) and receives platform ops (host-view).
 *
 * Implementations:
 *   - QuickJsEngine — QuickJS-ng via rquickjs (default, all platforms)
 *   - JsCoreEngine — JavaScriptCore (Apple platforms, saves 1 MB on iOS per D-5)
 */
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A JS value that can be passed between Rust and the JS engine.
///
/// Mirrors the TypeScript `JsonValue` type in the command protocol
/// (`packages/contract/src/protocol.ts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsValue>),
    Object(HashMap<String, JsValue>),
}

impl From<Value> for JsValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => JsValue::Null,
            Value::Bool(b) => JsValue::Boolean(b),
            Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => JsValue::String(s),
            Value::Array(arr) => JsValue::Array(arr.into_iter().map(JsValue::from).collect()),
            Value::Object(obj) => JsValue::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, JsValue::from(v)))
                    .collect(),
            ),
        }
    }
}

impl From<JsValue> for Value {
    fn from(v: JsValue) -> Self {
        match v {
            JsValue::Null => Value::Null,
            JsValue::Boolean(b) => Value::Bool(b),
            JsValue::Number(n) => Value::Number(
                serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            JsValue::String(s) => Value::String(s),
            JsValue::Array(arr) => Value::Array(arr.into_iter().map(Value::from).collect()),
            JsValue::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Error returned by engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Engine not initialized")]
    NotInitialized,
    #[error("Script evaluation failed: {0}")]
    ScriptError(String),
    #[error("Function call failed: {0}")]
    CallError(String),
    #[error("Value conversion failed: {0}")]
    ConversionError(String),
    #[error("Engine internal error: {0}")]
    Internal(String),
}

/// Convert host mutation errors into engine errors (used by the runtime
/// adapter with `?`).
impl From<angnative_host_core::mutation::MutationError> for EngineError {
    fn from(e: angnative_host_core::mutation::MutationError) -> Self {
        EngineError::Internal(e.to_string())
    }
}

/// The engine-neutral JS runtime trait.
///
/// Platform shells depend on this trait, not on a specific engine. This keeps
/// the TS side engine-agnostic (spec D5).
pub trait JsEngine {
    /// Evaluate a JavaScript/TypeScript source string.
    ///
    /// Returns the result as a `JsValue`, or an `EngineError` on failure.
    fn eval(&mut self, source: &str) -> Result<JsValue, EngineError>;

    /// Call a registered JS function by name with the given arguments.
    fn call_function(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue, EngineError>;

    /// Register a Rust closure as a JS function callable from scripts.
    ///
    /// `func` receives the arguments as `JsValue`s and returns a `JsValue`.
    fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(Vec<JsValue>) -> Result<JsValue, EngineError> + 'static,
        Self: Sized;

    /// Set a global variable in the JS engine.
    fn set_global(&mut self, name: &str, value: JsValue) -> Result<(), EngineError>;

    /// Get a global variable from the JS engine.
    fn get_global(&self, name: &str) -> Option<JsValue>;

    /// Run garbage collection if the engine supports it.
    fn gc(&mut self) -> Result<(), EngineError>;

    /// Get engine memory usage in bytes (for diagnostics / size budget R8).
    fn memory_usage(&self) -> Option<usize>;

    /// Engine name for logging/diagnostics (e.g. "QuickJS-ng", "JavaScriptCore").
    fn engine_name(&self) -> &'static str;
}
