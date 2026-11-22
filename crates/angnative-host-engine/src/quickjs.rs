use rquickjs::function::Rest;
/**
 * quickjs.rs — QuickJS-ng (rquickjs) implementation of the JsEngine trait.
 *
 * Default JS engine per identity D-5. Runs on all platforms (Android, iOS,
 * Desktop). Lightweight (~1 MB stripped).
 *
 * API verified against docs.rs/rquickjs/0.5.1:
 *   - Value::new_undefined / new_null / new_bool / new_number
 *   - String::from_str / Object::new / Array::new + set
 *   - Exception::throw_message for errors
 *   - Runtime::run_gc for GC, Context::with for entry
 *
 * `register_function(name, f)` exposes `f` directly as a JS global function
 * named `name`, so no dispatcher indirection is needed. QjsValue borrows the
 * Ctx lifetime, so all conversions happen inside `Context::with`.
 */
use rquickjs::{Array, Context, Exception, Function, Object, Runtime, Value as QjsValue};
use std::sync::Arc;

use crate::engine::{EngineError, JsEngine, JsValue};

/// QuickJS-ng engine implementation.
///
/// rquickjs contexts are thread-affine; this engine is used on its owning host
/// thread and the engine-neutral trait deliberately does not promise Send/Sync.
pub struct QuickJsEngine {
    runtime: Runtime,
    context: Context,
}

impl QuickJsEngine {
    /// Create a new QuickJS-ng engine.
    pub fn new() -> Result<Self, EngineError> {
        let runtime = Runtime::new().map_err(|e| EngineError::Internal(e.to_string()))?;
        let context = Context::full(&runtime).map_err(|e| EngineError::Internal(e.to_string()))?;
        Ok(Self { runtime, context })
    }
}

impl Default for QuickJsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create QuickJS engine")
    }
}

impl JsEngine for QuickJsEngine {
    fn eval(&mut self, source: &str) -> Result<JsValue, EngineError> {
        self.context.with(|ctx| {
            let result: QjsValue = ctx
                .eval(source)
                .map_err(|e| EngineError::ScriptError(e.to_string()))?;
            Ok(qjs_value_to_js_value(&result))
        })
    }

    fn call_function(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue, EngineError> {
        let owned_args: Vec<JsValue> = args.to_vec();
        let owned_name = name.to_string();
        self.context.with(move |ctx| {
            let qjs_args: Vec<QjsValue> = owned_args
                .into_iter()
                .map(|a| js_value_to_qjs_value(&ctx, a))
                .collect::<Result<Vec<_>, _>>()?;
            let func: Function = ctx
                .globals()
                .get(owned_name.as_str())
                .map_err(|_| EngineError::CallError(format!("Function not found: {owned_name}")))?;
            let mut call_args = rquickjs::function::Args::new(ctx.clone(), qjs_args.len());
            for arg in qjs_args {
                call_args
                    .push_arg(arg)
                    .map_err(|e| EngineError::ConversionError(e.to_string()))?;
            }
            let result: QjsValue = func
                .call_arg(call_args)
                .map_err(|e| EngineError::CallError(e.to_string()))?;
            Ok(qjs_value_to_js_value(&result))
        })
    }

    fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(Vec<JsValue>) -> Result<JsValue, EngineError> + 'static,
    {
        let func = Arc::new(func);
        let owned_name = name.to_string();
        // expose the Rust closure directly as a JS global function
        let result: Result<(), EngineError> = self.context.with(|ctx| {
            let js_fn = Function::new(
                ctx.clone(),
                move |ctx: rquickjs::Ctx,
                      Rest(args): Rest<QjsValue>|
                      -> Result<JsValue, rquickjs::Error> {
                    let js_args: Vec<JsValue> = args.iter().map(qjs_value_to_js_value).collect();
                    match func(js_args) {
                        Ok(value) => Ok(value),
                        Err(e) => Err(Exception::throw_message(&ctx, &e.to_string())),
                    }
                },
            )
            .map_err(|e| EngineError::Internal(e.to_string()))?;
            ctx.globals()
                .set(owned_name.as_str(), js_fn)
                .map_err(|e| EngineError::Internal(e.to_string()))
        });
        if let Err(e) = result {
            log::error!("Failed to register JS function {name}: {e}");
        }
    }

    fn set_global(&mut self, name: &str, value: JsValue) -> Result<(), EngineError> {
        let owned_name = name.to_string();
        self.context.with(|ctx| {
            let qjs_value = js_value_to_qjs_value(&ctx, value)?;
            ctx.globals()
                .set(owned_name.as_str(), qjs_value)
                .map_err(|e| EngineError::Internal(e.to_string()))
        })
    }

    fn get_global(&self, name: &str) -> Option<JsValue> {
        let owned_name = name.to_string();
        self.context.with(|ctx| {
            let value: QjsValue = ctx.globals().get(owned_name.as_str()).ok()?;
            Some(qjs_value_to_js_value(&value))
        })
    }

    fn drain_microtasks(&mut self) -> Result<u32, EngineError> {
        let mut total = 0u32;
        for _ in 0..1024 {
            // JS-level queueMicrotask shim (engine-neutral API).
            let queued = match self.eval(
                r#"(function () {
                    if (typeof _angnative_microtasks === 'undefined') return 0;
                    var q = _angnative_microtasks;
                    _angnative_microtasks = [];
                    for (var i = 0; i < q.length; i++) q[i]();
                    return q.length;
                })()"#,
            )? {
                JsValue::Number(n) => n as u32,
                _ => 0,
            };

            // Native QuickJS Promise jobs — required for async AOT bundles
            // whose bootstrap uses `await import(...)` / `.then(...)`.
            let mut jobs = 0u32;
            while self.runtime.is_job_pending() {
                match self.runtime.execute_pending_job() {
                    Ok(true) => jobs += 1,
                    Ok(false) | Err(_) => break,
                }
                if jobs >= 1024 {
                    break;
                }
            }

            total += queued + jobs;
            if queued == 0 && jobs == 0 {
                break;
            }
        }
        Ok(total)
    }

    fn gc(&mut self) -> Result<(), EngineError> {
        self.runtime.run_gc();
        Ok(())
    }

    fn memory_usage(&self) -> Option<usize> {
        // Runtime::memory_usage() returns JSMemoryUsage; the platform shell
        // can measure RSS externally for the size budget (R8).
        None
    }

    fn engine_name(&self) -> &'static str {
        "QuickJS-ng"
    }
}

impl<'js> rquickjs::IntoJs<'js> for JsValue {
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<QjsValue<'js>> {
        js_value_to_qjs_value(ctx, self).map_err(|e| Exception::throw_message(ctx, &e.to_string()))
    }
}

/// Convert a rquickjs Value to our JsValue. Reads only; no Ctx needed.
fn qjs_value_to_js_value(value: &QjsValue<'_>) -> JsValue {
    qjs_value_to_js_value_at(value, 0)
}

/// Maximum depth walked when converting a JS value into the owned `JsValue`
/// tree. JS values can be cyclic (e.g. an RxJS `Subscription`, which references
/// its own parent/children). rquickjs cannot represent cycles and an unbounded
/// walk would recurse until the native stack overflows, so deeper values are
/// truncated to `Null` instead of crashing the runtime.
const MAX_JS_CONVERSION_DEPTH: usize = 16;

fn qjs_value_to_js_value_at(value: &QjsValue<'_>, depth: usize) -> JsValue {
    if value.is_undefined() || value.is_null() {
        return JsValue::Null;
    }
    if let Some(b) = value.as_bool() {
        return JsValue::Boolean(b);
    }
    if let Some(n) = value.as_number() {
        return JsValue::Number(n);
    }
    if let Some(s) = value.as_string() {
        return s.to_string().map(JsValue::String).unwrap_or(JsValue::Null);
    }
    if depth >= MAX_JS_CONVERSION_DEPTH {
        return JsValue::Null;
    }
    if value.is_array() {
        if let Some(arr) = value.as_array() {
            let mut out = Vec::new();
            for i in 0..arr.len() {
                let item: Result<QjsValue, _> = arr.get(i);
                out.push(
                    item.map(|v| qjs_value_to_js_value_at(&v, depth + 1))
                        .unwrap_or(JsValue::Null),
                );
            }
            return JsValue::Array(out);
        }
    }
    if let Some(obj) = value.as_object() {
        let mut map = std::collections::HashMap::new();
        for k in obj.keys::<rquickjs::String>().flatten() {
            if let Ok(ks) = k.to_string() {
                let val: Result<QjsValue, _> = obj.get(k.clone());
                if let Ok(v) = val {
                    map.insert(ks, qjs_value_to_js_value_at(&v, depth + 1));
                }
            }
        }
        return JsValue::Object(map);
    }
    JsValue::Null
}

/// Convert our JsValue to a rquickjs Value. Must be called inside
/// `Context::with` — the returned value borrows the Ctx lifetime.
fn js_value_to_qjs_value<'js>(
    ctx: &rquickjs::Ctx<'js>,
    value: JsValue,
) -> Result<QjsValue<'js>, EngineError> {
    let err = |e: rquickjs::Error| EngineError::ConversionError(e.to_string());
    match value {
        JsValue::Null => Ok(QjsValue::new_undefined(ctx.clone())),
        JsValue::Boolean(b) => Ok(QjsValue::new_bool(ctx.clone(), b)),
        JsValue::Number(n) => Ok(QjsValue::new_number(ctx.clone(), n)),
        JsValue::String(s) => {
            let js_str = rquickjs::String::from_str(ctx.clone(), &s).map_err(err)?;
            Ok(js_str.into())
        }
        JsValue::Array(arr) => {
            let js_arr = Array::new(ctx.clone()).map_err(err)?;
            for (i, v) in arr.into_iter().enumerate() {
                let qv = js_value_to_qjs_value(ctx, v)?;
                js_arr.set(i, qv).map_err(err)?;
            }
            Ok(js_arr.into())
        }
        JsValue::Object(obj) => {
            let js_obj = Object::new(ctx.clone()).map_err(err)?;
            for (k, v) in obj {
                let qv = js_value_to_qjs_value(ctx, v)?;
                js_obj.set(k.as_str(), qv).map_err(err)?;
            }
            Ok(js_obj.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quickjs_engine_eval() {
        let mut engine = QuickJsEngine::new().unwrap();
        let result = engine.eval("1 + 2").unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }

    #[test]
    fn quickjs_engine_string() {
        let mut engine = QuickJsEngine::new().unwrap();
        let result = engine.eval("'hello'").unwrap();
        assert_eq!(result, JsValue::String("hello".to_string()));
    }

    #[test]
    fn quickjs_engine_register_and_call() {
        let mut engine = QuickJsEngine::new().unwrap();

        engine.register_function("add", |args| {
            if args.len() != 2 {
                return Err(EngineError::CallError("add expects 2 args".to_string()));
            }
            match (&args[0], &args[1]) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a + b)),
                _ => Err(EngineError::CallError("add expects numbers".to_string())),
            }
        });

        let result = engine
            .call_function("add", &[JsValue::Number(5.0), JsValue::Number(7.0)])
            .unwrap();
        assert_eq!(result, JsValue::Number(12.0));
    }

    #[test]
    fn quickjs_engine_set_get_global() {
        let mut engine = QuickJsEngine::new().unwrap();
        engine.set_global("myValue", JsValue::Number(42.0)).unwrap();

        let result = engine.eval("myValue").unwrap();
        assert_eq!(result, JsValue::Number(42.0));

        let retrieved = engine.get_global("myValue").unwrap();
        assert_eq!(retrieved, JsValue::Number(42.0));
    }

    #[test]
    fn quickjs_engine_gc() {
        let mut engine = QuickJsEngine::new().unwrap();
        assert!(engine.gc().is_ok());
    }

    #[test]
    fn quickjs_engine_cyclic_value_does_not_overflow() {
        let mut engine = QuickJsEngine::new().unwrap();
        // A self-referential object graph must be truncated, not crash the
        // native stack (rquickjs cannot represent cycles).
        let result = engine.eval(r#"var o = { name: 'cycle' }; o.self = o; o;"#).unwrap();
        match result {
            JsValue::Object(map) => assert_eq!(
                map.get("name"),
                Some(&JsValue::String("cycle".to_string()))
            ),
            other => panic!("expected truncated object, got {other:?}"),
        }
    }

    #[test]
    fn engine_name() {
        let engine = QuickJsEngine::new().unwrap();
        assert_eq!(engine.engine_name(), "QuickJS-ng");
    }
}
