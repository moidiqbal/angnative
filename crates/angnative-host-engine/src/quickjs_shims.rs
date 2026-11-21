/**
 * quickjs_shims.rs — Minimal DOM and Platform shims for QuickJS.
 *
 * Provides the absolute minimum for Angular to run, using the JsEngine trait
 * API (eval, set_global, register_function).
 *
 * IMPORTANT (rquickjs 0.5.1 limitation): any *cyclic* JS object graph (e.g.
 * `el.parentNode = parent; parent.children.push(el)` or `window.self = window`)
 * causes a stack overflow during GC/Drop — confirmed in both debug and release
 * builds, at 1 MB and 8 MB stacks. Therefore the DOM tree is stored in a FLAT,
 * ID-BASED registry (`_domById`, `_domParent`, `_domChildren`, ...). Element
 * objects hold only a numeric `_domId`; relationships are computed on demand by
 * getters. `self`/`globalThis` are exposed as GETTERS, never stored references.
 */
use crate::engine::JsEngine;

/// Install minimal DOM and platform shims using the JsEngine trait API.
pub fn install_dom_shims<E: JsEngine>(engine: &mut E) -> Result<(), crate::engine::EngineError> {
    // Native console backends must exist before the `console` shim calls them.
    register_console_functions(engine)?;

    // Flat DOM registry + shared prototype. All relationships are stored by
    // numeric id — never as object-to-object references — to avoid the
    // rquickjs cyclic-GC stack overflow (see module docs).
    engine.eval(r#"
        // ES2022 helpers used by Angular 22 bundles that QuickJS-ng lacks.
        if (typeof Object.hasOwn !== 'function') {
            Object.hasOwn = function (obj, prop) {
                return Object.prototype.hasOwnProperty.call(obj, prop);
            };
        }

        if (typeof FinalizationRegistry === 'undefined') {
            globalThis.FinalizationRegistry = function () {};
            globalThis.FinalizationRegistry.prototype.register = function () {};
            globalThis.FinalizationRegistry.prototype.unregister = function () {};
            globalThis.FinalizationRegistry.prototype.cleanupSome = function () {};
        }
        if (typeof WeakRef === 'undefined') {
            globalThis.WeakRef = function (value) { this._value = value; };
            globalThis.WeakRef.prototype.deref = function () { return this._value; };
        }

        // Minimal i18n support: Angular AOT emits $localize tagged templates.
        // Translations are keyed the same way Angular's localize package uses
        // (message parts joined by \uFFFD<index>\uFFFD placeholders).
        if (typeof globalThis.$localize === 'undefined') {
            var _angnativeTranslations = {};
            globalThis.$localize = function (parts) {
                var key = parts[0];
                for (var k = 1; k < parts.length; k++) {
                    key += '\uFFFD' + (k - 1) + '\uFFFD' + parts[k];
                }
                var out = _angnativeTranslations[key];
                if (out === undefined) {
                    out = '';
                    for (var k = 0; k < parts.length; k++) out += parts[k];
                }
                for (var p = 1; p < arguments.length; p++) {
                    var marker = '\uFFFD' + (p - 1) + '\uFFFD';
                    while (out.indexOf(marker) >= 0) {
                        out = out.replace(marker, String(arguments[p]));
                    }
                }
                return out;
            };
            globalThis.$localize.loadTranslations = function (translations) {
                for (var key in translations) {
                    _angnativeTranslations[key] = translations[key];
                }
            };
        }

        // Angular's dev-mode component timing calls performance.mark/measure.
        if (typeof performance === 'undefined') {
            globalThis.performance = {
                mark: function () {},
                measure: function () {},
                now: function () { return 0; }
            };
        }

        // Single native event entry point. Rust dispatch_event calls this for
        // each listener id; the Angular renderer installs the hook that maps
        // the id back to its component callback.
        if (typeof globalThis.na_onEvent === 'undefined') {
            globalThis.na_onEvent = function (listenerId, payloadJson) {
                var hook = globalThis.__angnativeDispatchListener;
                if (typeof hook !== 'function') return 0;
                hook(listenerId, payloadJson);
                return 1;
            };
        }

        // Microtask queue — zoneless Angular change detection needs this.
        // A JS-level queue keeps the engine-neutral JsEngine trait simple and
        // avoids cyclic object graphs in QuickJS.
        var _angnative_microtasks = [];
        function queueMicrotask(callback) {
            if (typeof callback === 'function') _angnative_microtasks.push(callback);
        }

        var _domById = {};
        var _domParent = {};
        var _domChildren = {};
        var _domAttrs = {};
        var _domListeners = {};
        var _domNextId = 1;

        function _domNodeId(v) {
            return (v && typeof v === 'object' && typeof v._domId === 'number')
                ? v._domId : null;
        }

        var _domProto = {
            setAttribute: function(name, value) {
                var id = this._domId, key = String(name), val = String(value);
                _domAttrs[id][key] = val;
                if (key === 'id') this.id = val;
                if (key === 'class') this.className = val;
            },
            getAttribute: function(name) {
                var a = _domAttrs[this._domId][String(name)];
                return a === undefined ? null : a;
            },
            removeAttribute: function(name) {
                delete _domAttrs[this._domId][String(name)];
            },
            hasAttribute: function(name) {
                return Object.prototype.hasOwnProperty.call(
                    _domAttrs[this._domId], String(name));
            },
            appendChild: function(child) {
                var cid = _domNodeId(child);
                if (cid === null) return child;
                var old = _domParent[cid];
                if (old !== undefined && old !== null) {
                    var prev = _domChildren[old], pi = prev.indexOf(cid);
                    if (pi >= 0) prev.splice(pi, 1);
                }
                _domParent[cid] = this._domId;
                _domChildren[this._domId].push(cid);
                return child;
            },
            removeChild: function(child) {
                var cid = _domNodeId(child);
                if (cid === null) return child;
                var arr = _domChildren[this._domId], i = arr.indexOf(cid);
                if (i >= 0) { arr.splice(i, 1); delete _domParent[cid]; }
                return child;
            },
            insertBefore: function(newNode, refNode) {
                var cid = _domNodeId(newNode);
                if (cid === null) return newNode;
                var old = _domParent[cid];
                if (old !== undefined && old !== null) {
                    var prev = _domChildren[old], pi = prev.indexOf(cid);
                    if (pi >= 0) prev.splice(pi, 1);
                }
                var arr = _domChildren[this._domId];
                var rid = _domNodeId(refNode);
                var ri = rid === null ? -1 : arr.indexOf(rid);
                if (ri >= 0) arr.splice(ri, 0, cid); else arr.push(cid);
                _domParent[cid] = this._domId;
                return newNode;
            },
            addEventListener: function(type, handler) {
                var m = _domListeners[this._domId], key = String(type);
                if (!m[key]) m[key] = [];
                m[key].push(handler);
            },
            removeEventListener: function(type, handler) {
                var m = _domListeners[this._domId], key = String(type);
                if (!m[key]) return false;
                var i = m[key].indexOf(handler);
                if (i >= 0) { m[key].splice(i, 1); return true; }
                return false;
            },
            dispatchEvent: function(event) {
                var m = _domListeners[this._domId];
                var ls = m[event.type] || [];
                event.target = event.target || this;
                event.currentTarget = this;
                for (var i = 0; i < ls.length; i++) ls[i].call(this, event);
                return !event.defaultPrevented;
            },
            // Angular's SharedStylesHost calls doc.head.querySelectorAll(...),
            // so elements need these too (not just `document`).
            querySelectorAll: function(selector) {
                var out = [], nodes = [];
                _domWalk(this._domId, nodes);
                for (var i = 0; i < nodes.length; i++) {
                    if (_domMatches(nodes[i], selector)) out.push(nodes[i]);
                }
                return out;
            },
            querySelector: function(selector) {
                var nodes = [];
                _domWalk(this._domId, nodes);
                for (var i = 0; i < nodes.length; i++) {
                    if (_domMatches(nodes[i], selector)) return nodes[i];
                }
                return null;
            }
        };

        Object.defineProperty(_domProto, 'parentNode', { get: function() {
            var p = _domParent[this._domId];
            return (p === undefined || p === null) ? null : _domById[p];
        }});
        Object.defineProperty(_domProto, 'firstChild', { get: function() {
            var c = _domChildren[this._domId];
            return c.length ? _domById[c[0]] : null;
        }});
        Object.defineProperty(_domProto, 'lastChild', { get: function() {
            var c = _domChildren[this._domId];
            return c.length ? _domById[c[c.length - 1]] : null;
        }});
        Object.defineProperty(_domProto, 'nextSibling', { get: function() {
            var p = _domParent[this._domId];
            if (p === undefined || p === null) return null;
            var s = _domChildren[p], i = s.indexOf(this._domId);
            return (i >= 0 && i + 1 < s.length) ? _domById[s[i + 1]] : null;
        }});
        Object.defineProperty(_domProto, 'previousSibling', { get: function() {
            var p = _domParent[this._domId];
            if (p === undefined || p === null) return null;
            var s = _domChildren[p], i = s.indexOf(this._domId);
            return (i > 0) ? _domById[s[i - 1]] : null;
        }});
        Object.defineProperty(_domProto, 'children', { get: function() {
            return _domChildren[this._domId].map(function(id) { return _domById[id]; });
        }});
        Object.defineProperty(_domProto, 'childNodes', { get: function() {
            return _domChildren[this._domId].map(function(id) { return _domById[id]; });
        }});
        Object.defineProperty(_domProto, 'ownerDocument', { get: function() {
            return (typeof document !== 'undefined') ? document : null;
        }});
        Object.defineProperty(_domProto, 'textContent', {
            get: function() {
                var out = '', c = _domChildren[this._domId];
                for (var i = 0; i < c.length; i++) {
                    var n = _domById[c[i]];
                    out += (n.nodeValue !== null && n.nodeValue !== undefined)
                        ? n.nodeValue : n.textContent;
                }
                return out;
            },
            set: function(v) {
                var id = this._domId, c = _domChildren[id];
                for (var i = 0; i < c.length; i++) delete _domById[c[i]];
                _domChildren[id] = [];
                if (v !== null && v !== undefined && v !== '') {
                    var t = _domMake('#text', 3);
                    t.nodeValue = String(v);
                    _domParent[t._domId] = id;
                    _domChildren[id].push(t._domId);
                }
            }
        });

        function _domMake(tag, nodeType) {
            var id = _domNextId++;
            var el = Object.create(_domProto);
            el._domId = id;
            el.tagName = String(tag).toUpperCase();
            el.nodeType = nodeType;
            el.nodeValue = null;
            el.style = {};
            el.className = '';
            el.id = '';
            _domById[id] = el;
            _domChildren[id] = [];
            _domAttrs[id] = {};
            _domListeners[id] = {};
            return el;
        }

        function _domWalk(id, out) {
            var kids = _domChildren[id] || [];
            for (var i = 0; i < kids.length; i++) {
                out.push(_domById[kids[i]]);
                _domWalk(kids[i], out);
            }
        }

        function _domMatches(el, sel) {
            if (!sel) return false;
            var s = String(sel);
            if (s.charAt(0) === '#') {
                var want = s.slice(1);
                return el.id === want || el.getAttribute('id') === want;
            }
            if (s.charAt(0) === '.') {
                var cls = s.slice(1);
                return el.className.split(/\s+/).indexOf(cls) >= 0;
            }
            return el.tagName === s.toUpperCase();
        }
    "#)?;

    // document — real tree built on top of the flat registry.
    engine.eval(r#"
        var document = _domMake('#document', 9);
        document.documentElement = _domMake('html', 1);
        document.head = _domMake('head', 1);
        document.body = _domMake('body', 1);

        _domParent[document.documentElement._domId] = document._domId;
        _domChildren[document._domId].push(document.documentElement._domId);
        _domParent[document.head._domId] = document.documentElement._domId;
        _domChildren[document.documentElement._domId].push(document.head._domId);
        _domParent[document.body._domId] = document.documentElement._domId;
        _domChildren[document.documentElement._domId].push(document.body._domId);

        document.createElement = function(tag) { return _domMake(tag, 1); };
        document.createTextNode = function(text) {
            var t = _domMake('#text', 3);
            t.nodeValue = String(text);
            return t;
        };
        document.createComment = function(text) {
            var c = _domMake('#comment', 8);
            c.nodeValue = String(text);
            return c;
        };
        document.getElementById = function(id) {
            var want = String(id);
            for (var key in _domById) {
                var el = _domById[key];
                if (el.id === want || el.getAttribute('id') === want) return el;
            }
            return null;
        };
        document.getElementsByTagName = function(tag) {
            var out = [], all = [];
            _domWalk(document._domId, all);
            var want = String(tag).toUpperCase();
            for (var i = 0; i < all.length; i++) {
                if (all[i].tagName === want) out.push(all[i]);
            }
            return out;
        };
        document.querySelector = function(selector) {
            var all = [];
            _domWalk(document._domId, all);
            for (var i = 0; i < all.length; i++) {
                if (_domMatches(all[i], selector)) return all[i];
            }
            return null;
        };
        document.querySelectorAll = function(selector) {
            var out = [], all = [];
            _domWalk(document._domId, all);
            for (var i = 0; i < all.length; i++) {
                if (_domMatches(all[i], selector)) out.push(all[i]);
            }
            return out;
        };
    "#)?;

    // console
    engine.eval(r#"
        var console = {
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
        // Angular's zoneless scheduler uses an rAF-vs-setTimeout race. Drive
        // rAF from the same queue the host drains after each event, otherwise
        // change detection never ticks in QuickJS.
        var _angnativeRafId = 1;
        var _angnativeRafQueue = new Map();
        function requestAnimationFrame(callback) {
            var id = _angnativeRafId++;
            var run = function () {
                if (!_angnativeRafQueue.has(id)) return;
                _angnativeRafQueue.delete(id);
                callback(0);
            };
            _angnativeRafQueue.set(id, run);
            queueMicrotask(run);
            return id;
        }
        function cancelAnimationFrame(id) {
            _angnativeRafQueue.delete(id);
        }
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
        var _storage = {
            _data: new Map(),
            getItem(key) { return this._data.get(key) ?? null; },
            setItem(key, value) { this._data.set(key, value); },
            removeItem(key) { this._data.delete(key); },
            clear() { this._data.clear(); },
            get length() { return this._data.size; },
            key(index) { return Array.from(this._data.keys())[index] ?? null; },
        };
        var localStorage = Object.create(_storage);
        var sessionStorage = Object.create(_storage);
    "#)?;

    // window / globalThis — defined LAST so every referenced identifier
    // (console, timers, Event*, fetch, storage, document) already exists.
    // `self`/`globalThis` are GETTERS: storing a self-reference would create a
    // cyclic object graph and overflow rquickjs GC (see module docs).
    engine.eval(r#"
        var window = {
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
            document: document,
            queueMicrotask: queueMicrotask,
        };
        Object.defineProperty(window, 'self', { get: function() { return window; } });
        Object.defineProperty(window, 'window', { get: function() { return window; } });
        Object.defineProperty(window, 'globalThis', { get: function() { return window; } });
    "#)?;

    Ok(())
}

/// Install module system (require/define/System) using JsEngine trait API.
pub fn install_module_system<E: JsEngine>(engine: &mut E) -> Result<(), crate::engine::EngineError> {
    engine.eval(r#"
        var _moduleCache = new Map();
        function require(id) {
            if (_moduleCache.has(id)) return _moduleCache.get(id);
            var exports = {};
            _moduleCache.set(id, exports);
            return exports;
        }

        function define(id, factory) {
            var exports = factory();
            return exports;
        }
        define.amd = true;

        var System = {
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

    fn installed() -> QuickJsEngine {
        let mut engine = QuickJsEngine::new().unwrap();
        install_dom_shims(&mut engine).unwrap();
        engine
    }

    #[test]
    fn test_install_dom_shims() {
        let mut engine = installed();

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
    fn test_self_is_a_safe_getter() {
        let mut engine = installed();
        // No cyclic object graph: `self`/`window`/`globalThis` are computed.
        assert_eq!(
            engine.eval("window.self === window && window.window === window").unwrap(),
            crate::engine::JsValue::Boolean(true)
        );
    }

    #[test]
    fn test_dom_tree_operations() {
        let mut engine = installed();
        let result = engine.eval(r#"
            (function () {
                var parent = document.createElement('div');
                var child = document.createElement('span');
                document.body.appendChild(parent);
                parent.id = 'host';
                parent.className = 'a b';
                child.setAttribute('id', 'target');
                child.setAttribute('class', 'clickable');
                parent.appendChild(child);

                var r = [];
                r.push(child.parentNode === parent ? 1 : 0);
                r.push(parent.firstChild === child ? 1 : 0);
                r.push(parent.lastChild === child ? 1 : 0);
                r.push(child.nextSibling === null ? 1 : 0);
                r.push(child.previousSibling === null ? 1 : 0);
                r.push(parent.children.length === 1 ? 1 : 0);
                r.push(child.getAttribute('id') === 'target' ? 1 : 0);
                r.push(child.hasAttribute('class') ? 1 : 0);
                r.push(document.getElementById('target') === child ? 1 : 0);
                r.push(document.querySelectorAll('.clickable').length === 1 ? 1 : 0);
                r.push(document.querySelector('SPAN') === child ? 1 : 0);

                var fired = 0;
                child.addEventListener('click', function (e) { fired++; });
                child.dispatchEvent(new Event('click'));
                r.push(fired === 1 ? 1 : 0);

                parent.removeChild(child);
                r.push(parent.children.length === 0 && child.parentNode === null ? 1 : 0);
                return r.join(',');
            })()
        "#).unwrap();
        assert_eq!(
            result,
            crate::engine::JsValue::String("1,1,1,1,1,1,1,1,1,1,1,1,1".to_string())
        );
    }

    #[test]
    fn test_dom_text_content_and_insert_before() {
        let mut engine = installed();
        let result = engine.eval(r#"
            (function () {
                var root = document.createElement('div');
                var a = document.createElement('a');
                var b = document.createElement('b');
                root.appendChild(b);
                root.insertBefore(a, b);
                var ok = root.firstChild === a && root.lastChild === b;
                ok = ok && a.nextSibling === b && b.previousSibling === a;
                root.textContent = 'hello';
                ok = ok && root.childNodes.length === 1;
                ok = ok && root.textContent === 'hello';
                return ok;
            })()
        "#).unwrap();
        assert_eq!(result, crate::engine::JsValue::Boolean(true));
    }

    #[test]
    fn test_queue_microtask_and_drain() {
        let mut engine = installed();
        engine.eval(r#"
            var _order = [];
            queueMicrotask(function () { _order.push('first'); });
            queueMicrotask(function () {
                _order.push('second');
                queueMicrotask(function () { _order.push('nested'); });
            });
        "#).unwrap();

        // The drain loop keeps going until the queue is empty, so the nested
        // microtask runs in the same call.
        assert_eq!(engine.drain_microtasks().unwrap(), 3);
        assert_eq!(engine.drain_microtasks().unwrap(), 0);
        assert_eq!(
            engine.eval("_order.join(',')").unwrap(),
            crate::engine::JsValue::String("first,second,nested".to_string())
        );
    }

    #[test]
    fn test_localize_i18n_translations() {
        let mut engine = installed();
        engine.eval(r#"
            $localize.loadTranslations({
                "Hello \uFFFD0\uFFFD!": "Assalam-o-Alaikum \uFFFD0\uFFFD!"
            });
            var _greeting = $localize`Hello ${'AngNative'}!`;
        "#).unwrap();
        assert_eq!(
            engine.eval("_greeting").unwrap(),
            crate::engine::JsValue::String("Assalam-o-Alaikum AngNative!".to_string())
        );
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
