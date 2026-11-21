/**
 * quickjs_shims.rs — Minimal DOM and Platform shims for QuickJS.
 *
 * Provides only the absolute minimum for Angular to run.
 * Uses the JsEngine trait API (eval, set_global, register_function).
 */
use crate::engine::JsEngine;

/// Install minimal DOM and platform shims using the JsEngine trait API.
pub fn install_dom_shims<E: JsEngine>(engine: &mut E) -> Result<(), crate::engine::EngineError> {
    // Native console backends must exist before the `console` shim calls them.
    register_console_functions(engine)?;

    // document
    engine.eval(r#"
        const document = {
            createElement: function(tag) {
                const el = {
                    tagName: tag.toUpperCase(),
                    nodeType: 1,
                    style: {},
                    className: '',
                    id: '',
                    setAttribute: function(name, value) {},
                    getAttribute: function(name) { return null; },
                    removeAttribute: function(name) {},
                    appendChild: function(child) { return null; },
                    removeChild: function(child) { return null; },
                    insertBefore: function(newNode, refNode) { return null; },
                    addEventListener: function(event, handler) {},
                    removeEventListener: function(event, handler) {},
                    dispatchEvent: function(event) { return true; },
                    parentNode: null,
                    firstChild: null,
                    lastChild: null,
                    nextSibling: null,
                    previousSibling: null,
                    ownerDocument: document,
                };
                return el;
            },
            createTextNode: function(text) {
                return {
                    nodeType: 3,
                    nodeValue: text,
                    textContent: text,
                    parentNode: null,
                };
            },
            createComment: function(text) {
                return {
                    nodeType: 8,
                    nodeValue: text,
                    textContent: text,
                    parentNode: null,
                };
            },
            getElementById: function(id) { return null; },
            querySelector: function(selector) { return null; },
            querySelectorAll: function(selector) { return []; },
            body: null,
            documentElement: null,
            head: null,
        };
    "#)?;
    
    // console
    engine.eval(r#"
        const console = {
            log: function(...args) {
                let out = '';
                for (let i = 0; i < args.length; i++) {
                    out += args[i] + ' ';
                }
                _angnative_console_log(out);
            },
            warn: function(...args) {
                let out = '';
                for (let i = 0; i < args.length; i++) {
                    out += args[i] + ' ';
                }
                _angnative_console_warn(out);
            },
            error: function(...args) {
                let out = '';
                for (let i = 0; i < args.length; i++) {
                    out += args[i] + ' ';
                }
                _angnative_console_error(out);
            },
        };
    "#)?;
    
    // Timer stubs (minimal)
    engine.eval(r#"
        function setTimeout(handler, delay) { return 1; }
        function clearTimeout(id) {}
        function setInterval(handler, delay) { return 2; }
        function clearInterval(id) {}
        function requestAnimationFrame(callback) { return 1; }
        function cancelAnimationFrame(id) {}
    "#)?;
    
    // EventTarget, Event, CustomEvent
    engine.eval(r#"
        class EventTarget {
            constructor() {
                this._listeners = new Map();
            }
            addEventListener(type, handler, options) {
                if (!this._listeners.has(type)) this._listeners.set(type, []);
                this._listeners.get(type).push({ handler, options });
            }
            removeEventListener(type, handler, options) {
                if (!this._listeners.has(type)) return;
                const listeners = this._listeners.get(type);
                const idx = listeners.findIndex(l => l.handler === handler && l.options === options);
                if (idx >= 0) listeners.splice(idx, 1);
            }
            dispatchEvent(event) {
                if (!this._listeners.has(event.type)) return true;
                for (const l of this._listeners.get(event.type)) {
                    l.handler(event);
                }
                return !event.defaultPrevented;
            }
        }
        
        function Event(type, options) {
            this.type = type;
            this.bubbles = options?.bubbles ?? false;
            this.cancelable = options?.cancelable ?? false;
            this.defaultPrevented = false;
            this.target = null;
            this.currentTarget = null;
            this.preventDefault = function() { this.defaultPrevented = true; };
            this.stopPropagation = function() {};
            this.stopImmediatePropagation = function() {};
            this.timeStamp = 0;
        }
        
        function CustomEvent(type, options) {
            Event.call(this, type, options);
            this.detail = options?.detail ?? null;
        }
        CustomEvent.prototype = Object.create(Event.prototype);
    "#)?;
    
    // fetch stub
    engine.eval(r#"
        function fetch(url, options) {
            return Promise.reject(new Error('fetch not implemented'));
        }
    "#)?;
    
    // localStorage/sessionStorage stubs
    engine.eval(r#"
        const _storage = {
            _data: new Map(),
            getItem(key) { return this._data.get(key) ?? null; },
            setItem(key, value) { this._data.set(key, value); },
            removeItem(key) { this._data.delete(key); },
            clear() { this._data.clear(); },
            get length() { return this._data.size; },
            key(index) { return Array.from(this._data.keys())[index] ?? null; },
        };
        const localStorage = Object.create(_storage);
        const sessionStorage = Object.create(_storage);
    "#)?;
    
    // window / globalThis — defined LAST so every referenced identifier
    // (console, timers, Event*, fetch, storage) already exists. Defining it
    // first caused a QuickJS exception on undefined identifiers.
    // NOTE: `window.self = window` / `window.globalThis = window` are
    // deliberately omitted: cyclic object graphs trigger a stack overflow in
    // rquickjs 0.5.1 (debug builds) during GC/Drop. Angular code that relies on
    // `window.self` must use `globalThis` instead until this is resolved.
    engine.eval(r#"
        const window = {
            console: console,
            setTimeout: setTimeout,
            clearTimeout: clearTimeout,
            setInterval: setInterval,
            clearInterval: clearInterval,
            requestAnimationFrame: requestAnimationFrame,
            cancelAnimationFrame: cancelAnimationFrame,
            localStorage: localStorage,
            sessionStorage: sessionStorage,
            fetch: fetch,
            EventTarget: EventTarget,
            Event: Event,
            CustomEvent: CustomEvent,
        };
    "#)?;
    
    Ok(())
}

/// Install module system (require/define/System) using JsEngine trait API.
pub fn install_module_system<E: JsEngine>(engine: &mut E) -> Result<(), crate::engine::EngineError> {
    engine.eval(r#"
        const _moduleCache = new Map();
        function require(id) {
            if (_moduleCache.has(id)) return _moduleCache.get(id);
            const exports = {};
            _moduleCache.set(id, exports);
            return exports;
        }
        
        function define(id, factory) {
            const exports = factory();
            return exports;
        }
        define.amd = true;
        
        const System = {
            register: function(deps, factory) {}
        };
    "#)?;
    
    Ok(())
}

/// Register native console functions
pub fn register_console_functions<E: JsEngine>(engine: &mut E) -> Result<(), crate::engine::EngineError> {
    engine.register_function("_angnative_console_log", |args| {
        let mut out = String::new();
        for arg in args {
            out += &format!("{:?}", arg);
            out.push(' ');
        }
        println!("[JS Console] {}", out.trim());
        Ok(crate::engine::JsValue::Null)
    });
    
    engine.register_function("_angnative_console_warn", |args| {
        let mut out = String::new();
        for arg in args {
            out += &format!("{:?}", arg);
            out.push(' ');
        }
        eprintln!("[JS Console Warn] {}", out.trim());
        Ok(crate::engine::JsValue::Null)
    });
    
    engine.register_function("_angnative_console_error", |args| {
        let mut out = String::new();
        for arg in args {
            out += &format!("{:?}", arg);
            out.push(' ');
        }
        eprintln!("[JS Console Error] {}", out.trim());
        Ok(crate::engine::JsValue::Null)
    });
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickjs::QuickJsEngine;
    
    #[test]
    fn test_install_dom_shims() {
        let mut engine = QuickJsEngine::new().unwrap();
        install_dom_shims(&mut engine).unwrap();
        
        let result = engine.eval("typeof window").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("object".to_string()));
        
        let result = engine.eval("typeof document").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("object".to_string()));
        
        let result = engine.eval("typeof setTimeout").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("function".to_string()));
        
        let result = engine.eval("typeof Event").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("function".to_string()));
    }
    
    #[test]
    fn test_install_module_system() {
        let mut engine = QuickJsEngine::new().unwrap();
        install_module_system(&mut engine).unwrap();
        
        let result = engine.eval("typeof require").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("function".to_string()));
        
        let result = engine.eval("typeof define").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("function".to_string()));
        
        let result = engine.eval("typeof System").unwrap();
        assert_eq!(result, crate::engine::JsValue::String("object".to_string()));
    }
}