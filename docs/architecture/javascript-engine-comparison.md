# JavaScript Engine Runtime Research

> **UPDATE 2026-11-23:** This document originally recommended V8/deno_core as
> the primary engine, pending "Phase G" benchmarks that were never run. That
> recommendation is **superseded**. The finalized decision is in
> [ADR-javascript-runtime-selection.md](ADR-javascript-runtime-selection.md):
> **retain the QuickJS family, upgrade the engine, minify/bytecode the bundles**.
> Rationale: 4–10× smaller binary, no iOS JIT penalty, and Angular compatibility
> is already proven via shims on emulator and a physical device. The V8 section
> below is retained as historical research, not as a current recommendation.

## Executive Summary (original research)

This document presents research findings on JavaScript engine options for AngNative's embedded Angular execution runtime. The research was conducted to inform the architectural decision for the primary execution engine.

## Engines Evaluated

### 1. QuickJS-NG (Current: rquickjs)

**Version correction (2026-11-23):** rquickjs 0.5.1 actually vendors QuickJS with
`VERSION = 2021-03-27` — the original Bellard QuickJS, NOT QuickJS-NG 2024.
QuickJS-ng's current release is v0.18.0 (1 Nov 2026), which contains years of
memory-safety and performance fixes. See the ADR for the upgrade decision.

**Capabilities Verified**:
- ES2023 support (partial)
- ES Modules (via `import()` dynamic import)
- CommonJS `require()` (via custom module loader)
- Promise support
- Async/await
- TypedArrays, DataView
- WebAssembly (experimental)
- Bytecode compilation (`qjsbc`)

**Limitations for Angular**:
- No native Zone.js support (requires polyfill)
- No native timer implementation (setTimeout, setInterval)
- No Promise queue microtask integration (requires manual drain)
- No DOM APIs (document, window, EventTarget)
- No fetch/XHR
- No typed module resolution
- Single-threaded, no Worker support
- GC not incremental (pause times scale with heap)

**Thread Affinity**: Strict - Context must be used on creation thread

**Binary Size**: ~1.5MB (stripped, LTO)

**Memory**: ~2-4MB baseline

**rquickjs 0.5 Status**:
- Thread-safe runtime/context creation
- `register_function` for native callbacks
- `Module::eval` for ESM
- `Context::with` for scoped execution
- Promise integration via `Promise::new` + `resolve`/`reject`
- GC: `Runtime::run_gc()`
- Memory: `Runtime::memory_usage()` (JS heap only)

### 2. QuickJS-NG (Direct: qjs crate)

**Differences from rquickjs**:
- Lower-level bindings
- Direct access to QuickJS C API
- Manual memory management
- More control over module loading
- No automatic Rust<->JS value conversion

**Migration Effort**: Medium-High (rewrite all engine bindings)

### 3. Hermes (Facebook)

**Version**: Hermes 0.12+

**Capabilities**:
- Bytecode precompilation (hbc)
- ES2022+ support
- Hermes bytecode format
- JIT on Android (AOT on iOS)
- Generational GC
- Snapshots
- `require()` / ES modules

**Android Integration**:
- Native JNI bindings included
- Prebuilt AAR available
- Used in production (React Native, Expo, Shopify)

**iOS Constraints**:
- No JIT on iOS (Apple policy)
- AOT compilation required
- Larger binary than QuickJS

**Binary Size**: ~3-4MB (stripped)

**Rust Bindings**: `hermes-rs` (unofficial, incomplete)

**Angular Compatibility**: Partial - missing Zone.js, some ES2023 features

### 4. V8 via deno_core

**Version**: V8 12.x via deno_core 0.300+

**Capabilities**:
- Full V8 engine (Chrome's JS engine)
- Complete ES2024 support
- WebAssembly (full)
- V8 Snapshots (startup < 10ms)
- Isolates (multiple contexts)
- Native Promise, async/await
- Microtask queue (native)
- Timer integration (via event loop)
- Inspector protocol (devtools)

**deno_core Features**:
- Module loader trait
- Snapshot support
- Ops (async native functions)
- Resource management
- Event loop integration (tokio)
- Permission system
- TypeScript transpilation (built-in)

**iOS Constraints**:
- No JIT (Apple policy) - V8 runs in interpreter mode
- `rusty_v8` provides JIT-less build
- Larger binary (~15-20MB)

**Binary Size**: ~8-15MB (deno_core + V8, stripped)

**Rust Integration**: Excellent - first-class deno_core API

**Angular Compatibility**: **Best** - Angular targets V8/Chrome

### 5. JavaScriptCore (iOS/macOS only)

**Platform**: Apple platforms only

**Capabilities**:
- Full ES2023
- Native iOS/macOS integration
- JIT on macOS, interpreter on iOS
- WebAssembly
- Bytecode caching

**Cross-platform**: Not viable (Apple-only)

**Rust Bindings**: `javascriptcore-sys` (low-level)

---

## Angular Runtime Requirements Analysis

### Angular 22+ Platform Abstractions Required

Based on Angular source analysis (`packages/platform-browser`, `packages/core`):

1. **PlatformRef** - Application bootstrap
2. **RendererFactory2** + **Renderer2** - DOM/Node mutations
3. **DomAdapter** - Platform abstraction layer
4. **NgZone** / **ZonelessChangeDetection** - Change detection scheduling
5. **AnimationDriver** - Animations
6. **Document** - Document reference
7. **EventManager** - Event handling
8. **HammerGestureConfig** - Gestures
9. **Location** - URL routing
10. **PlatformLocation** - Platform location
11. **Testability** - Testing hooks

### Minimal Platform Requirements for AngNative

Angular's `PlatformRef` requires:

```typescript
interface PlatformRef {
  bootstrapModuleFactory<T>(moduleFactory: NgModuleFactory<T>, options?: BootstrapOptions): Promise<NgModuleRef<T>>;
  destroy(): void;
  onDestroy: Observable<void>;
}
```

For `bootstrapApplication` (standalone):
- `EnvironmentProviders[]`
- `provideZoneChangeDetection()` or `provideZonelessChangeDetection()`
- `RendererFactory2` provider
- `DomAdapter` provider (or custom)

### Zoneless Change Detection (Angular 18+)

Angular 18+ supports zoneless via:
```typescript
provideZonelessChangeDetection()
```

Requires:
- `scheduleMicroTask` implementation
- `EventTarget` for event dispatch
- No `Zone.js` needed

### Angular Signals (Angular 16+)

Signals work without Zone.js:
- `signal()`, `computed()`, `effect()`
- Automatic change detection scheduling
- Works with zoneless

---

## Engine Comparison Matrix

| Criterion | QuickJS-NG | Hermes | V8 (deno_core) |
|-----------|------------|--------|----------------|
| **Angular Compatibility** | ⚠️ Partial (shims needed) | ⚠️ Partial | ✅ **Native** |
| **Zone.js Required** | Yes (polyfill) | Yes (polyfill) | **No (zoneless works)** |
| **Binary Size** | ~1.5MB | ~3-4MB | ~8-15MB |
| **Startup (cold)** | ~20ms | ~15ms | ~5ms (snapshot) |
| **Startup (warm)** | ~5ms | ~8ms | ~1ms (snapshot) |
| **Memory (idle)** | 2-4MB | 4-8MB | 8-15MB |
| **GC Pause (10MB heap)** | ~5ms | ~2ms | ~1ms (incremental) |
| **Thread Affinity** | Strict | Strict | Isolates (flexible) |
| **iOS Support** | ✅ Interpreter | ✅ AOT | ⚠️ JIT-less |
| **Android JIT** | ❌ Interpreter | ✅ JIT | ⚠️ JIT-less |
| **Rust Integration** | rquickjs (good) | hermes-rs (partial) | deno_core (excellent) |
| **Snapshot Support** | Bytecode only | ✅ Full | ✅ **V8 Snapshots** |
| **Debugging** | Basic | Good | **DevTools** |
| **Angular Zone.js** | Polyfill needed | Polyfill needed | **Not needed** |
| **ES Modules** | Dynamic import | ✅ Native | ✅ Native |
| **TypeScript** | No | No | Built-in (deno_core) |
| **Long-term Maintenance** | Active (QuickJS-NG) | Active (Meta) | **Active (Google/Deno)** |

---

## Recommendation (HISTORICAL — superseded, see ADR)

### Primary (old proposal, NOT adopted): **V8 via deno_core** 

**Rationale**:
1. **Angular runs natively** - Angular's primary target is V8/Chrome
2. **Zoneless works without polyfills** - No Zone.js shim needed
3. **V8 Snapshots** - Sub-10ms startup with pre-warmed Angular runtime
4. **Excellent Rust integration** - deno_core is designed for embedding
5. **DevTools debugging** - Critical for Angular development
6. **Long-term support** - Google + Deno maintain V8/deno_core
6. **TypeScript transpilation** - Built into deno_core if needed

**Trade-off**: ~8-15MB binary vs ~2MB QuickJS. For mobile, this is acceptable (Angular apps are 5-20MB anyway).

### Fallback: **QuickJS-NG (retained)**

If V8 proves problematic:
- Retain current rquickjs integration
- Add minimal shims for Zone.js, timers, Promise queue
- Accept longer Angular bootstrap time
- Binary stays ~2MB

### Experimental: **Hermes**

Evaluate only if:
- Binary size is critical constraint (<3MB)
- React Native-style architecture preferred
- Willing to maintain hermes-rs bindings

---

## Conclusion (HISTORICAL)

The original conclusion — "Recommend V8 via deno_core, decide after Phase G" —
was never enacted: no V8 PoC or Phase G benchmarks were produced. The final,
evidence-backed decision is recorded in
[ADR-javascript-runtime-selection.md](ADR-javascript-runtime-selection.md):
**QuickJS retained, engine upgraded, bundles minified/bytecoded.**

**Next Steps**:
1. Phase C: Complete Angular AOT + QuickJS-NG proof-of-concept
2. Phase G: Benchmark V8 vs QuickJS-NG on physical Android device
3. ADR: Final engine selection based on evidence

---

## References

- Angular Source: https://github.com/angular/angular (v22.x)
- NativeScript Angular: https://github.com/NativeScript/angular
- QuickJS-NG: https://github.com/quickjs-ng/quickjs
- rquickjs: https://github.com/DelSkayn/rquickjs
- Hermes: https://github.com/facebook/hermes
- deno_core: https://github.com/denoland/deno_core
- rusty_v8: https://github.com/denoland/rusty_v8
- Angular Zoneless: https://angular.dev/guide/zoneless