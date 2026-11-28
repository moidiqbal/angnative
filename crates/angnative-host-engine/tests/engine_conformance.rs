/**
 * engine_conformance.rs — shared `JsEngine` conformance suite.
 *
 * Tier A (API shape) must hold for every implementation: the real QuickJS
 * engine, the in-repo ReferenceEngine test double, and the optional StubEngine
 * contributor template. Passing Tier A does NOT mean an engine executes JS.
 *
 * Tier B (JS-language semantics) applies only to engines that actually parse
 * JavaScript — today QuickJS only. Keep the tiers separate so a non-executing
 * double can never be misquoted as engine parity.
 */
use angnative_host_engine::engine::JsEngine;
use angnative_host_engine::quickjs::QuickJsEngine;
use angnative_host_engine::{EngineError, JsValue, ReferenceEngine};

/// Tier A: trait API shape, no JavaScript evaluation involved.
fn assert_api_shape<E: JsEngine>(mut engine: E) {
    // Globals round-trip through the engine API (not through `eval`).
    engine
        .set_global("conformance_key", JsValue::String("value".to_string()))
        .expect("set_global must succeed");
    assert_eq!(
        engine.get_global("conformance_key"),
        Some(JsValue::String("value".to_string()))
    );
    // QuickJS resolves a missing global to JS `undefined` (mapped to Null);
    // recorder doubles return None. Both are valid API-shape outcomes.
    assert!(matches!(
        engine.get_global("missing_key"),
        None | Some(JsValue::Null)
    ));

    // Registered functions round-trip and receive owned argument values.
    engine.register_function("echo_first", |args| {
        Ok(args.into_iter().next().unwrap_or(JsValue::Null))
    });
    assert_eq!(
        engine
            .call_function("echo_first", &[JsValue::Number(42.0)])
            .expect("registered function must be callable"),
        JsValue::Number(42.0)
    );
    assert!(matches!(
        engine.call_function("not_registered", &[]),
        Err(EngineError::CallError(_))
    ));

    // Lifecycle helpers stay idempotent and bounded on a clean realm.
    engine.gc().expect("gc must succeed");
    engine.gc().expect("gc must be idempotent");
    let _ = engine.memory_usage();
    assert!(!engine.engine_name().is_empty());
    let drained = engine
        .drain_microtasks()
        .expect("clean realm must not fail microtask drain");
    assert!(drained <= 1024, "microtask drain must be bounded");
}

#[test]
fn tier_a_api_shape_quickjs() {
    assert_api_shape(QuickJsEngine::new().expect("QuickJS engine must initialize"));
}

#[test]
fn tier_a_api_shape_reference_engine() {
    assert_api_shape(ReferenceEngine::new());
}

#[cfg(feature = "engine-stub")]
#[test]
fn tier_a_api_shape_stub_engine() {
    use angnative_host_engine::stub_engine::StubEngine;
    assert_api_shape(StubEngine::new());
}

#[test]
fn tier_a_engine_names_are_unique() {
    #[allow(unused_mut)] // mutated only when the engine-stub feature is enabled
    let mut names = vec![
        QuickJsEngine::new().unwrap().engine_name(),
        ReferenceEngine::new().engine_name(),
    ];
    #[cfg(feature = "engine-stub")]
    names.push(angnative_host_engine::stub_engine::StubEngine::new().engine_name());
    for (i, name) in names.iter().enumerate() {
        assert!(!name.is_empty());
        assert!(!names[i + 1..].contains(name), "duplicate engine name: {name}");
    }
}

/// Tier B: JS-language semantics, QuickJS only. A non-executing double is
/// never required to pass this tier.
#[test]
fn tier_b_quickjs_evaluates_and_rejects_invalid_syntax() {
    let mut engine = QuickJsEngine::new().expect("QuickJS engine must initialize");
    assert_eq!(engine.eval("1 + 2").unwrap(), JsValue::Number(3.0));
    assert_eq!(
        engine.eval("'conformance'").unwrap(),
        JsValue::String("conformance".to_string())
    );
    assert!(matches!(
        engine.eval("this is not valid JavaScript"),
        Err(EngineError::ScriptError(_))
    ));
}
