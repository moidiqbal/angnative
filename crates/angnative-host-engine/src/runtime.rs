/**
 * runtime.rs — AngNative JS runtime API exposed to scripts.
 *
 * Exposes the native view operations (createElement, setStyle, appendChild,
 * etc.) to JavaScript/TypeScript code running in the engine. JS calls these
 * functions, which translate to command-protocol mutations sent to the host
 * (host-core MutationEngine + host-view OpEmitter).
 *
 * This is the bridge between the embedded JS engine and the Rust host tree.
 */
use serde_json::Value;

use crate::engine::{EngineError, JsEngine, JsValue};

/// Configuration for the AngNative JS runtime.
#[derive(Debug, Clone, Default)]
pub struct RuntimeConfig {
    /// Whether to enable debug mode (stack traces, etc.)
    pub debug: bool,
}

/// The AngNative JS runtime. Wraps a `JsEngine` and exposes native view
/// operations to JS code.
pub struct AngNativeRuntime<E: JsEngine> {
    engine: E,
    adapter: MutationEngineAdapter,
    config: RuntimeConfig,
}

impl<E: JsEngine> AngNativeRuntime<E> {
    /// Create a new runtime with the given engine and config.
    pub fn new(engine: E, config: RuntimeConfig) -> Self {
        Self {
            engine,
            adapter: MutationEngineAdapter::new(),
            config,
        }
    }

    /// Install the AngNative runtime API into the JS engine's global scope.
    ///
    /// After calling this, JS code can use:
    ///   `na.createElement(tag)` → creates a native node, returns node id
    ///   `na.appendChild(parentId, childId)` → appends child
    ///   `na.setStyle(nodeId, key, value)` → sets a style
    ///   `na.setProperty(nodeId, key, value)` → sets a property
    ///   `na.setTextContent(nodeId, text)` → sets text
    ///   `na.flush()` → flushes pending mutations
    pub fn install_api(&mut self) -> Result<(), EngineError> {
        let create_adapter = self.adapter.clone();
        let append_adapter = self.adapter.clone();
        let style_adapter = self.adapter.clone();
        let property_adapter = self.adapter.clone();
        let text_adapter = self.adapter.clone();
        let flush_adapter = self.adapter.clone();

        // createElement(tag) → nodeId
        self.engine
            .register_function("na_createElement", move |args| {
                if args.len() != 1 {
                    return Err(EngineError::CallError(
                        "na.createElement expects 1 arg".to_string(),
                    ));
                }
                match &args[0] {
                    JsValue::String(tag) => {
                        let mut lock = create_adapter.lock();
                        let node_id = lock.create_element(tag)?;
                        Ok(JsValue::Number(node_id as f64))
                    }
                    _ => Err(EngineError::CallError(
                        "na.createElement expects a string tag".to_string(),
                    )),
                }
            });

        // appendChild(parentId, childId)
        self.engine
            .register_function("na_appendChild", move |args| {
                if args.len() != 2 {
                    return Err(EngineError::CallError(
                        "na.appendChild expects 2 args".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (JsValue::Number(p), JsValue::Number(c)) => {
                        let mut lock = append_adapter.lock();
                        lock.append_child(*p as u64, *c as u64)?;
                        Ok(JsValue::Null)
                    }
                    _ => Err(EngineError::CallError(
                        "na.appendChild expects two numbers".to_string(),
                    )),
                }
            });

        // setStyle(nodeId, key, value)
        self.engine.register_function("na_setStyle", move |args| {
            if args.len() != 3 {
                return Err(EngineError::CallError(
                    "na.setStyle expects 3 args".to_string(),
                ));
            }
            match (&args[0], &args[1], &args[2]) {
                (JsValue::Number(id), JsValue::String(key), JsValue::String(value)) => {
                    let mut lock = style_adapter.lock();
                    lock.set_style(*id as u64, key, value)?;
                    Ok(JsValue::Null)
                }
                _ => Err(EngineError::CallError(
                    "na.setStyle expects (number, string, string)".to_string(),
                )),
            }
        });

        // setProperty(nodeId, key, value)
        self.engine
            .register_function("na_setProperty", move |args| {
                if args.len() != 3 {
                    return Err(EngineError::CallError(
                        "na.setProperty expects 3 args".to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (JsValue::Number(id), JsValue::String(key), value) => {
                        let mut lock = property_adapter.lock();
                        lock.set_property(*id as u64, key, serde_json::Value::from(value.clone()))?;
                        Ok(JsValue::Null)
                    }
                    _ => Err(EngineError::CallError(
                        "na.setProperty expects (number, string, value)".to_string(),
                    )),
                }
            });

        // setTextContent(nodeId, text)
        self.engine
            .register_function("na_setTextContent", move |args| {
                if args.len() != 2 {
                    return Err(EngineError::CallError(
                        "na.setTextContent expects 2 args".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (JsValue::Number(id), JsValue::String(text)) => {
                        let mut lock = text_adapter.lock();
                        lock.set_text_content(*id as u64, text)?;
                        Ok(JsValue::Null)
                    }
                    _ => Err(EngineError::CallError(
                        "na.setTextContent expects (number, string)".to_string(),
                    )),
                }
            });

        // flush() → flush pending mutations
        self.engine.register_function("na_flush", move |_args| {
            let mut lock = flush_adapter.lock();
            lock.flush()?;
            Ok(JsValue::Null)
        });

        // Create a convenience `na` namespace object. register_function() above
        // exposed each na_* function directly as a JS global, so call them by name.
        let na_js = r#"
            var na = {
                createElement: function(tag) { return na_createElement(tag); },
                appendChild: function(parentId, childId) { return na_appendChild(parentId, childId); },
                setStyle: function(nodeId, key, value) { return na_setStyle(nodeId, key, value); },
                setProperty: function(nodeId, key, value) { return na_setProperty(nodeId, key, value); },
                setTextContent: function(nodeId, text) { return na_setTextContent(nodeId, text); },
                flush: function() { return na_flush(); }
            };
        "#;
        self.engine.eval(na_js)?;

        Ok(())
    }

    /// Evaluate a JS source string with the AngNative API available.
    pub fn eval(&mut self, source: &str) -> Result<JsValue, EngineError> {
        self.engine.eval(source)
    }

    /// Get the underlying engine (for advanced use).
    pub fn engine(&mut self) -> &mut E {
        &mut self.engine
    }

    /// Get a reference to the adapter (for inspecting the tree).
    pub fn adapter(&self) -> &MutationEngineAdapter {
        &self.adapter
    }

    /// Return the runtime options supplied at construction.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

/// Thread-safe adapter bridging JS calls to the Rust host tree.
///
/// Wraps a MutationEngine and provides the methods called by registered JS
/// functions. Uses interior mutability so closures can capture it.
#[derive(Clone, Default)]
pub struct MutationEngineAdapter {
    inner: std::sync::Arc<std::sync::Mutex<MutationEngineAdapterInner>>,
}

#[derive(Default)]
struct MutationEngineAdapterInner {
    engine: angnative_host_core::mutation::MutationEngine,
    frame_id: u64,
}

impl MutationEngineAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MutationEngineAdapterInner> {
        self.inner.lock().unwrap()
    }
}

impl MutationEngineAdapterInner {
    pub fn create_element(&mut self, tag: &str) -> Result<u64, EngineError> {
        let node_type = match tag {
            "na-view" | "view" | "div" => "View",
            "na-text" | "text" | "span" | "p" => "Text",
            "na-button" | "button" => "Button",
            "na-image" | "image" | "img" => "Image",
            "na-scroll-view" | "scroll" | "scrollview" => "ScrollView",
            "na-text-input" | "text-input" | "input" => "TextInput",
            _ => "Custom",
        };

        // Allocate an id via the engine's public API
        let id = self.engine.allocate_id();
        self.engine
            .run_command(angnative_host_core::command::Command::CreateNode {
                id,
                node_type: node_type.to_string(),
            })?;
        Ok(id)
    }

    pub fn append_child(&mut self, parent_id: u64, child_id: u64) -> Result<(), EngineError> {
        // Compute the append position from the parent's current child count.
        let index = self
            .engine
            .get_node(parent_id)
            .map(|n| n.children.len())
            .unwrap_or(0);
        self.engine
            .run_command(angnative_host_core::command::Command::InsertChild {
                parent_id,
                child_id,
                index,
            })?;
        Ok(())
    }

    pub fn set_style(&mut self, node_id: u64, key: &str, value: &str) -> Result<(), EngineError> {
        self.engine
            .run_command(angnative_host_core::command::Command::SetStyle {
                id: node_id,
                key: key.to_string(),
                value: value.to_string(),
            })?;
        Ok(())
    }

    pub fn set_property(
        &mut self,
        node_id: u64,
        key: &str,
        value: Value,
    ) -> Result<(), EngineError> {
        self.engine
            .run_command(angnative_host_core::command::Command::SetProperty {
                id: node_id,
                key: key.to_string(),
                value,
            })?;
        Ok(())
    }

    pub fn set_text_content(&mut self, node_id: u64, text: &str) -> Result<(), EngineError> {
        self.engine
            .run_command(angnative_host_core::command::Command::SetTextContent {
                id: node_id,
                text: text.to_string(),
            })?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), EngineError> {
        self.frame_id += 1;
        // In a real implementation, this would flush the OpEmitter ops to
        // the platform shell via ViewFactory. For now, it's a no-op beyond
        // incrementing the frame id (mutations are applied immediately).
        Ok(())
    }
}
