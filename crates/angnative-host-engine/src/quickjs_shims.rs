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

        // --- History / Location shims for @angular/router -----------------
        // Flat state only (no object cycles — QuickJS cannot GC cycles here).
        var _angOrigin = 'http://localhost';
        var _histStack = [{ state: null, url: _angOrigin + '/' }];
        var _histIndex = 0;
        var _popListeners = [];

        function _angParseUrl(url) {
            var href = String(url || _angOrigin + '/');
            // Strip the origin so pathname/search/hash match browser behaviour.
            if (href.indexOf(_angOrigin) === 0) href = href.slice(_angOrigin.length) || '/';
            var hashIdx = href.indexOf('#');
            var hash = hashIdx >= 0 ? href.slice(hashIdx) : '';
            var noHash = hashIdx >= 0 ? href.slice(0, hashIdx) : href;
            var qIdx = noHash.indexOf('?');
            var search = qIdx >= 0 ? noHash.slice(qIdx) : '';
            var pathname = qIdx >= 0 ? noHash.slice(0, qIdx) : noHash;
            return { href: href, pathname: pathname, search: search, hash: hash };
        }

        // Standard URL shim for Angular router / serializer.
        // Required by Angular's isProtocolRelative check; without URL, every
        // route serializer throws and falls back to '/'.
        function URL(url, base) {
            var full = String(url);
            if (base && full.indexOf('://') < 0 && full.indexOf('//') !== 0) {
                var baseStr = String(base);
                if (full.charAt(0) === '/') {
                    var m = baseStr.match(/^([a-zA-Z]+:\/\/[^\/]+)/);
                    full = (m ? m[1] : baseStr) + full;
                } else {
                    full = baseStr.replace(/\/?$/, '/') + full;
                }
            }
            var m = full.match(/^([a-zA-Z]+:\/\/[^\/?#]+)?(\/[^?#]*)?(\?[^#]*)?(#.*)?$/);
            this.origin = (m && m[1]) || '';
            this.pathname = (m && m[2]) || '/';
            this.search = (m && m[3]) || '';
            this.hash = (m && m[4]) || '';
            this.href = full;
        }
        URL.prototype.toString = function () { return this.href; };
        globalThis.URL = URL;

        var _domLocation = Object.create(null);
        function _domSyncLocation() {
            var p = _angParseUrl(_histStack[_histIndex].url);
            _domLocation.href = p.href;
            _domLocation.pathname = p.pathname;
            _domLocation.search = p.search;
            _domLocation.hash = p.hash;
            _domLocation.origin = _angOrigin;
        }
        _domSyncLocation();

        function _domFirePop() {
            var event = { type: 'popstate', state: _histStack[_histIndex].state };
            for (var i = 0; i < _popListeners.length; i++) _popListeners[i](event);
        }

        var _domHistory = {
            get length() { return _histStack.length; },
            get state() { return _histStack[_histIndex].state; },
            scrollRestoration: 'auto',
            pushState: function (state, title, url) {
                _histStack = _histStack.slice(0, _histIndex + 1);
                var full = String(url).indexOf('http') === 0 ? String(url) : _angOrigin + String(url);
                _histStack.push({ state: state, url: full });
                _histIndex++;
                _domSyncLocation();
            },
            replaceState: function (state, title, url) {
                var full = String(url).indexOf('http') === 0 ? String(url) : _angOrigin + String(url);
                _histStack[_histIndex] = { state: state, url: full };
                _domSyncLocation();
            },
            back: function () {
                if (_histIndex > 0) { _histIndex--; _domSyncLocation(); _domFirePop(); }
            },
            forward: function () {
                if (_histIndex < _histStack.length - 1) { _histIndex++; _domSyncLocation(); _domFirePop(); }
            },
            go: function (delta) {
                var next = _histIndex + (delta || 0);
                if (next >= 0 && next < _histStack.length) {
                    _histIndex = next; _domSyncLocation(); _domFirePop();
                }
            }
        };
        globalThis.location = _domLocation;
        globalThis.history = _domHistory;

        // Angular Forms' DefaultValueAccessor asks the DOM adapter for the
        // user agent to decide composition-event mode. A flat primitive-only
        // object is enough; no object back-references (rquickjs GC limit).
        if (typeof globalThis.navigator === 'undefined') {
            globalThis.navigator = {
                userAgent: 'AngNative QuickJS',
                language: 'en-US',
                platform: 'AngNative'
            };
        }

        // Close the renderer -> native pipeline: the Angular renderer's flush
        // handler publishes batches through this hook. Route them into the
        // Host's real batch processor (registered by the runtime bridge).
        if (typeof globalThis.__angnativeDispatchBatch === 'undefined' &&
            typeof na_dispatchBatch === 'function') {
            globalThis.__angnativeDispatchBatch = function (batchJson) {
                var ops = na_dispatchBatch(batchJson);
                globalThis.__angnativeLastOps = ops;
                return ops;
            };
        }

        // --- UTF-8 codecs -------------------------------------------------
        // QuickJS-ng has no TextEncoder/TextDecoder. Angular's FetchBackend
        // always decodes the response body through TextDecoder, so both are
        // required for DI-based HttpClient. Pure functions, no object cycles.
        function _angUtf8Encode(str) {
            var s = String(str);
            var out = [];
            for (var i = 0; i < s.length; i++) {
                var c = s.charCodeAt(i);
                if (c < 0x80) {
                    out.push(c);
                } else if (c < 0x800) {
                    out.push(0xC0 | (c >> 6), 0x80 | (c & 0x3F));
                } else if (c >= 0xD800 && c <= 0xDBFF && i + 1 < s.length) {
                    var c2 = s.charCodeAt(++i);
                    var cp = 0x10000 + ((c - 0xD800) << 10) + (c2 - 0xDC00);
                    out.push(0xF0 | (cp >> 18), 0x80 | ((cp >> 12) & 0x3F),
                             0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F));
                } else {
                    out.push(0xE0 | (c >> 12), 0x80 | ((c >> 6) & 0x3F),
                             0x80 | (c & 0x3F));
                }
            }
            return new Uint8Array(out);
        }

        function _angUtf8Decode(input) {
            if (input === null || input === undefined) return '';
            var bytes = (input instanceof Uint8Array)
                ? input
                : new Uint8Array(input.buffer || input);
            var out = '';
            var i = 0;
            while (i < bytes.length) {
                var b = bytes[i++];
                var cp;
                if (b < 0x80) {
                    cp = b;
                } else if (b < 0xE0) {
                    cp = ((b & 0x1F) << 6) | (bytes[i++] & 0x3F);
                } else if (b < 0xF0) {
                    cp = ((b & 0x0F) << 12) | ((bytes[i++] & 0x3F) << 6) |
                         (bytes[i++] & 0x3F);
                } else {
                    cp = ((b & 0x07) << 18) | ((bytes[i++] & 0x3F) << 12) |
                         ((bytes[i++] & 0x3F) << 6) | (bytes[i++] & 0x3F);
                }
                out += String.fromCodePoint(cp);
            }
            return out;
        }

        if (typeof globalThis.TextEncoder === 'undefined') {
            globalThis.TextEncoder = function TextEncoder() {};
            globalThis.TextEncoder.prototype.encode = function (str) {
                return _angUtf8Encode(str);
            };
        }
        if (typeof globalThis.TextDecoder === 'undefined') {
            globalThis.TextDecoder = function TextDecoder(encoding) {
                this.encoding = encoding || 'utf-8';
            };
            globalThis.TextDecoder.prototype.decode = function (input) {
                return _angUtf8Decode(input);
            };
        }

        // --- fetch / HttpClient platform surface --------------------------
        // A host (or test) can register handlers. Default is a 404 response so
        // Angular's FetchBackend can initialize without a real network stack.
        //
        // Angular's FetchBackend does NOT call response.text()/json(): it reads
        // `response.body.getReader()` and decodes Uint8Array chunks through
        // TextDecoder. The response therefore exposes a minimal ReadableStream
        // shaped reader and a Headers-shaped plain object (enumerable header
        // names for `new HttpHeaders(obj)`, non-enumerable `get`).
        var _angFetchHandlers = [];
        globalThis.__angnativeRegisterFetchHandler = function (handler) {
            if (typeof handler === 'function') _angFetchHandlers.push(handler);
        };
        // Shared Headers-shaped prototype. `get` uses `this`, so instances hold
        // no closure back-reference to themselves (no GC cycle).
        var _angHeadersProto = {
            get: function (name) {
                var key = String(name).toLowerCase();
                for (var k in this) {
                    if (Object.prototype.hasOwnProperty.call(this, k) &&
                        k.toLowerCase() === key) return this[k];
                }
                return null;
            }
        };
        globalThis.fetch = function (url, options) {
            var opts = options || {};
            var request = {
                url: String(url),
                method: opts.method || 'GET',
                headers: opts.headers || {},
                body: opts.body === undefined ? null : opts.body
            };
            var result = null;
            for (var i = 0; i < _angFetchHandlers.length; i++) {
                result = _angFetchHandlers[i](request);
                if (result) break;
            }
            var status = result && result.status ? result.status : 404;
            var rawBody = result && result.body !== undefined ? result.body : '';
            var bodyText = typeof rawBody === 'string' ? rawBody : JSON.stringify(rawBody);
            var rawHeaders = (result && result.headers) || {};

            // Enumerable own string values so Angular's HttpHeaders(Object.entries)
            // path works; `get` lives on a shared prototype (never a closure
            // over the instance — that would create a GC cycle).
            var headerMap = Object.create(_angHeadersProto);
            for (var hk in rawHeaders) {
                if (!Object.prototype.hasOwnProperty.call(rawHeaders, hk)) continue;
                var hv = rawHeaders[hk];
                headerMap[String(hk)] = Array.isArray(hv) ? hv.join(', ') : String(hv);
            }
            if (headerMap['content-type'] === undefined &&
                headerMap['Content-Type'] === undefined) {
                headerMap['content-type'] = 'application/json';
            }

            var bytes = _angUtf8Encode(bodyText);
            var offset = 0;
            var reader = {
                read: function () {
                    if (offset >= bytes.length) {
                        return Promise.resolve({ done: true, value: undefined });
                    }
                    var chunk = bytes.slice(offset);
                    offset = bytes.length;
                    return Promise.resolve({ done: false, value: chunk });
                },
                cancel: function () {
                    offset = bytes.length;
                    return Promise.resolve();
                },
                releaseLock: function () {}
            };

            var response = {
                ok: status >= 200 && status < 300,
                status: status,
                statusText: (result && result.statusText) || '',
                url: String(url),
                redirected: false,
                type: 'basic',
                headers: headerMap,
                body: {
                    getReader: function () { return reader; },
                    cancel: function () { return reader.cancel(); }
                },
                text: function () { return Promise.resolve(bodyText); },
                json: function () { return Promise.resolve(JSON.parse(bodyText)); }
            };
            return Promise.resolve(response);
        };

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
            // Web Animations API (no-op driver) for @angular/animations.
            animate: function (keyframes, options) {
                var player = {
                    playState: 'running',
                    currentTime: 0,
                    play: function () { this.playState = 'running'; return this; },
                    pause: function () { this.playState = 'paused'; return this; },
                    finish: function () { this.playState = 'finished'; },
                    cancel: function () { this.playState = 'idle'; },
                    addEventListener: function () {},
                    removeEventListener: function () {}
                };
                // Resolve with undefined: resolving with `player` would create a
                // cyclic object graph and overflow rquickjs GC.
                player.finished = new Promise(function (resolve) {
                    queueMicrotask(function () {
                        player.playState = 'finished';
                        resolve(undefined);
                    });
                });
                return player;
            },
            getAnimations: function () { return []; },
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
        // Drive one-shot timers from the host-drained microtask queue. Angular
        // Router uses setTimeout for navigation scheduling; a no-op would leave
        // navigation pending forever.
        var _angTimerId = 1;
        var _angTimerQueue = new Map();
        function setTimeout(handler, delay) {
            var id = _angTimerId++;
            var run = function () {
                if (!_angTimerQueue.has(id)) return;
                _angTimerQueue.delete(id);
                handler();
            };
            _angTimerQueue.set(id, run);
            queueMicrotask(run);
            return id;
        }
        function clearTimeout(id) { _angTimerQueue.delete(id); }
        // RxJS asyncScheduler (used by Angular Router) flushes through
        // setInterval(0), so a no-op here leaves the navigation pipeline dead.
        // One-shot, NOT auto-repeating: an auto-repeating microtask chain never
        // drains and starves the host's bounded job loop. RxJS asyncScheduler
        // requests a fresh interval after each flush when it has more work.
        var _angIntervalId = 1;
        var _angIntervalQueue = new Map();
        function setInterval(handler, delay) {
            var id = _angIntervalId++;
            var run = function () {
                if (!_angIntervalQueue.has(id)) return;
                _angIntervalQueue.delete(id);
                handler();
            };
            _angIntervalQueue.set(id, run);
            queueMicrotask(run);
            return id;
        }
        function clearInterval(id) { _angIntervalQueue.delete(id); }
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

        // Flat, cycle-free AbortSignal and AbortController.
        // Listeners are stored in a root map keyed by numeric signal ID, never on
        // the signal instance itself, preventing cyclic GC stack overflows in rquickjs.
        var _abortSignalNextId = 1;
        var _abortListeners = new Map();

        function AbortSignal() {
            this._id = _abortSignalNextId++;
            this.aborted = false;
            this.reason = undefined;
            this.onabort = null;
        }
        AbortSignal.prototype.addEventListener = function(type, handler) {
            if (type !== 'abort' || typeof handler !== 'function') return;
            var list = _abortListeners.get(this._id);
            if (!list) {
                list = [];
                _abortListeners.set(this._id, list);
            }
            if (list.indexOf(handler) < 0) list.push(handler);
        };
        AbortSignal.prototype.removeEventListener = function(type, handler) {
            if (type !== 'abort') return;
            var list = _abortListeners.get(this._id);
            if (!list) return;
            var idx = list.indexOf(handler);
            if (idx >= 0) list.splice(idx, 1);
            if (list.length === 0) _abortListeners.delete(this._id);
        };
        AbortSignal.prototype.dispatchEvent = function(event) {
            if (event.type !== 'abort') return true;
            if (typeof this.onabort === 'function') {
                try { this.onabort(event); } catch (e) {}
            }
            var list = _abortListeners.get(this._id);
            if (list) {
                var copy = list.slice();
                for (var i = 0; i < copy.length; i++) {
                    try { copy[i](event); } catch (e) {}
                }
            }
            return true;
        };
        AbortSignal.prototype.throwIfAborted = function() {
            if (this.aborted) throw this.reason;
        };
        AbortSignal.abort = function(reason) {
            var s = new AbortSignal();
            s.aborted = true;
            s.reason = reason !== undefined ? reason : new Error('This operation was aborted');
            return s;
        };
        AbortSignal.timeout = function(delay) {
            var s = new AbortSignal();
            setTimeout(function() {
                if (s.aborted) return;
                s.aborted = true;
                s.reason = new Error('TimeoutError');
                s.dispatchEvent(new Event('abort'));
                _abortListeners.delete(s._id);
            }, delay);
            return s;
        };

        function AbortController() {
            this.signal = new AbortSignal();
        }
        AbortController.prototype.abort = function(reason) {
            if (this.signal.aborted) return;
            this.signal.aborted = true;
            this.signal.reason = reason !== undefined ? reason : new Error('This operation was aborted');
            this.signal.dispatchEvent(new Event('abort'));
            _abortListeners.delete(this.signal._id);
        };

        globalThis.AbortSignal = AbortSignal;
        globalThis.AbortController = AbortController;
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
            AbortController: AbortController,
            AbortSignal: AbortSignal,
            URL: URL,
            document: document,
            queueMicrotask: queueMicrotask,
            location: _domLocation,
            history: _domHistory,
            navigator: navigator,
            addEventListener: function (type, handler) {
                if (type === 'popstate') _popListeners.push(handler);
            },
            removeEventListener: function (type, handler) {
                if (type !== 'popstate') return;
                var i = _popListeners.indexOf(handler);
                if (i >= 0) _popListeners.splice(i, 1);
            }
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
    fn test_navigator_user_agent_shim() {
        let mut engine = installed();
        assert_eq!(
            engine.eval("window.navigator.userAgent").unwrap(),
            crate::engine::JsValue::String("AngNative QuickJS".to_string())
        );
        assert_eq!(
            engine.eval("navigator === window.navigator").unwrap(),
            crate::engine::JsValue::Boolean(true)
        );
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
    fn test_history_and_location_routing_shims() {
        let mut engine = installed();
        engine.eval(r#"
            var _popped = 'none';
            window.addEventListener('popstate', function (e) {
                _popped = e.state === null ? 'null' : JSON.stringify(e.state);
            });
            history.pushState(null, '', '/counter');
            var p1 = location.pathname;
            history.pushState({ id: 2 }, '', '/about?x=1');
            var p2 = location.pathname + location.search;
            history.back();
            var p3 = location.pathname + '|' + _popped;
        "#).unwrap();
        assert_eq!(
            engine.eval("[p1, p2, p3].join(',')").unwrap(),
            crate::engine::JsValue::String("/counter,/about?x=1,/counter|null".to_string())
        );
    }

    #[test]
    fn test_web_animations_noop_driver() {
        let mut engine = installed();
        engine.eval(r#"
            var el = document.createElement('div');
            var anim = el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 100 });
            var _before = anim.playState;
        "#).unwrap();
        assert_eq!(
            engine.eval("_before").unwrap(),
            crate::engine::JsValue::String("running".to_string())
        );
        engine.drain_microtasks().unwrap();
        assert_eq!(
            engine.eval("anim.playState").unwrap(),
            crate::engine::JsValue::String("finished".to_string())
        );
    }

    #[test]
    fn test_fetch_response_readable_stream_and_text_decoder() {
        let mut engine = installed();
        // Angular's FetchBackend reads response.body.getReader() and decodes
        // Uint8Array chunks through TextDecoder. Exercise that exact shape.
        engine.eval(r#"
            __angnativeRegisterFetchHandler(function (req) {
                return {
                    status: 201,
                    body: { message: 'héllo', method: req.method }
                };
            });
            var _fetchCheck = null;
            fetch('/api/x').then(function (r) {
                var reader = r.body.getReader();
                return reader.read().then(function (chunk) {
                    var text = new TextDecoder().decode(chunk.value);
                    var parsed = JSON.parse(text);
                    _fetchCheck = r.status + '|' + r.ok + '|' + parsed.message +
                        '|' + parsed.method + '|' +
                        r.headers.get('content-type');
                });
            });
        "#).unwrap();
        engine.drain_microtasks().unwrap();
        assert_eq!(
            engine.eval("_fetchCheck").unwrap(),
            crate::engine::JsValue::String(
                "201|true|héllo|GET|application/json".to_string()
            )
        );
    }

    #[test]
    fn test_text_encoder_decoder_roundtrip() {
        let mut engine = installed();
        let result = engine.eval(r#"
            (function () {
                var s = 'aé€𝄞z';
                var bytes = new TextEncoder().encode(s);
                return new TextDecoder().decode(bytes) === s &&
                    bytes.length === 11;
            })()
        "#).unwrap();
        assert_eq!(result, crate::engine::JsValue::Boolean(true));
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
    fn test_abort_controller_cycle_free() {
        let mut engine = installed();
        let res = engine.eval(r#"
            (function () {
                var ctrl = new AbortController();
                var sig = ctrl.signal;
                var abortedFired = 0;
                var handler = function (e) {
                    abortedFired++;
                };
                sig.addEventListener('abort', handler);
                var initial = sig.aborted;
                ctrl.abort('test reason');
                var after = sig.aborted;
                var reason = sig.reason;
                sig.removeEventListener('abort', handler);
                return [initial, after, abortedFired, reason].join(',');
            })()
        "#).unwrap();
        assert_eq!(
            res,
            crate::engine::JsValue::String("false,true,1,test reason".to_string())
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
