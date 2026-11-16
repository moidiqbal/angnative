# AngNative Architecture Blueprint

## 1. Executive Summary

**AngNative** is a high-performance cross-platform native framework that pairs the developer ergonomics of **Angular** (declarative templates, Signals, Dependency Injection, and reactive state) with the safety, raw speed, and memory efficiency of **Rust**.

Unlike hybrid Web-wrapping technologies (such as Cordova or Capacitor) which execute inside a mobile or desktop WebView, AngNative operates on the architectural principles pioneered by React Native and Flutter:
1. **The application logic** runs in an embedded JavaScript/TypeScript engine.
2. **The view hierarchy** is decoupled from browser DOM and expressed as a typed Native Shadow Tree.
3. **The layout calculation** (Flexbox/Grid) runs asynchronously in a high-performance Rust core using [Taffy](https://github.com/DioxusLabs/taffy).
4. **The final rendering** maps directly to native platform surfaces (Android Native Views / SurfaceView, Desktop native windows via Winit & WGPU, and Web semantic DOM).

---

## 2. High-Level Architecture Diagram

```
+------------------------------------------------------------------------+
|                      Angular Application Layer                         |
|     Standalone Components, Signals (signal(), computed(), effect())     |
|             Templates (<native-view>, <native-text>, etc.)             |
+------------------------------------------------------------------------+
                                   |
                                   v
+------------------------------------------------------------------------+
|                     @angnative/core (TypeScript)                       |
|           AngNativeRendererFactory2 & AngNativeRenderer2               |
|      Maintains NativeElement Tree & batches ViewMutation records        |
+------------------------------------------------------------------------+
                                   |
           (Direct In-Memory FFI / High-Speed Message Queue)
                                   v
+------------------------------------------------------------------------+
|                        Rust Core Runtime Engine                        |
|                                                                        |
|  +-----------------------+  +---------------------------------------+  |
|  |   angnative-compiler  |  |           angnative-layout            |  |
|  |  Rust-based TS/AST    |  |     Taffy Flexbox & CSS Grid          |  |
|  |  Parser (OXC / SWC)   |  |   Asynchronous Layout Calculation     |  |
|  +-----------------------+  +---------------------------------------+  |
|                                                                        |
|  +-----------------------+  +---------------------------------------+  |
|  |    angnative-core     |  |           angnative-runtime           |  |
|  | Shadow Tree Diffing   |  | Embedded JS/TS Engine (QuickJS/Deno)  |  |
|  | Batch Mutation Queue  |  |    Direct Rust-JS Memory Mapping      |  |
|  +-----------------------+  +---------------------------------------+  |
+------------------------------------------------------------------------+
                                   |
                                   v
+------------------------------------------------------------------------+
|                       Platform Output Adapters                         |
|                                                                        |
|   +-------------------+  +-------------------+  +------------------+   |
|   |   Android NDK     |  |   Desktop Host    |  |     Web DOM      |   |
|   |  (JNI / Native    |  |  (Windows, macOS, |  |  (@angnative/web |   |
|   |   Views / Surface)|  |   Linux via WGPU) |  |   HTML5 Adapter) |   |
|   +-------------------+  +-------------------+  +------------------+   |
+------------------------------------------------------------------------+
```

---

## 3. Core Subsystems

### 3.1 Angular Custom Renderer (`@angnative/core`)
Angular's architecture cleanly separates the framework's change detection and template reconciliation from DOM operations via `Renderer2` and `RendererFactory2`.

AngNative provides `AngNativeRenderer`:
- `createElement(name: string)`: Instantiates a `NativeElement` with a unique 64-bit integer ID (`node_id`).
- `appendChild(parent: NativeElement, child: NativeElement)`: Records an `InsertChild` mutation.
- `removeChild(parent: NativeElement, child: NativeElement)`: Records a `RemoveChild` mutation.
- `setProperty(el: NativeElement, name: string, value: any)`: Records an `UpdateProp` mutation.
- `listen(target, eventName, callback)`: Binds a native event subscription.

At the end of Angular's change detection tick, batched mutations are flushed to the Rust engine in a single FFI call.

### 3.2 Rust-Based Layout Engine (`angnative-layout`)
- Uses **Taffy**, the fastest W3C-compliant layout engine written in pure Rust.
- Calculates Flexbox (`flex-direction`, `justify-content`, `align-items`, `flex-grow`, `gap`) and Grid systems.
- Decoupled from the UI rendering thread: runs on a high-priority background thread, avoiding UI jank.

### 3.3 TypeScript Compiler in Rust (`angnative-compiler`)
- Integrates a native Rust TypeScript parser and transformer (based on OXC/SWC).
- Provides instant incremental transpilation of Angular TypeScript code without Node.js overhead.
- Generates optimized JavaScript byte arrays or bundles for the embedded runtime.

### 3.4 Embedded Runtime (`angnative-runtime`)
- Houses an embedded JavaScript engine (QuickJS for ultra-lightweight mobile/desktop deployments, or Deno Core/V8 for high-throughput environments).
- Exposes native Rust functions to the JS environment via zero-copy bindings.

### 3.5 Desktop Shell (`angnative-desktop`)
- Implemented in Rust using `winit` for cross-platform window management across Windows, macOS, and Linux.
- Supports GPU accelerated drawing via `wgpu` or direct native platform widgets.

### 3.6 Android Native Shell (`angnative-android`)
- Crate configured as a `cdylib` generating `libangnative.so`.
- Integrates with Android NDK using JNI.
- Dispatches calculated layout rectangles to Android `ViewGroup` hierarchy or renders directly onto an Android `SurfaceView`.

### 3.7 Web Adapter (`@angnative/web`)
- Provides Web compatibility by mapping `<native-view>` to `<div>`, `<native-text>` to `<span>`, and native flex styles to standard CSS.
- Enables complete code sharing between Mobile, Desktop, and Web.
