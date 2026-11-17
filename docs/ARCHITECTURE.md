# 🏗️ AngNative Architecture & System Design

**Status:** Canonical Reference (v1.0)  
**License:** MIT  
**Ecosystem Scope:** `@angnative/*` (TypeScript) · `angnative-*` (Rust)

---

## 1. Executive Overview

**AngNative** is an open-source, high-performance cross-platform application framework designed to render **modern Angular applications as true native UI widgets** (Android `View`s, iOS `UIView`s, Desktop native surfaces) with **zero WebViews**.

AngNative decouples the Angular application layer from the browser DOM by providing a custom `Renderer2` implementation. Instead of mutating HTML elements, template mutations are serialized into an efficient binary/JSON command buffer and dispatched across a zero-copy FFI boundary into a high-speed **Rust Host Engine**.

Layout calculation is handled asynchronously in Rust via **Taffy** (W3C Flexbox and CSS Grid), and the resulting layout rectangles are mounted onto native platform views.

---

## 2. High-Level Architecture Diagram

```
+-------------------------------------------------------------------------+
|                        Angular 19+ Application                          |
|         Standalone Components, Reactive Signals, Zoneless Runtime       |
|      Templates (<na-view>, <na-text>, <na-button>, <na-scroll-view>)    |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                      @angnative/core (TypeScript)                       |
|         AngNativeRendererFactory2 & Custom Angular Renderer2            |
|         DOM-Free NativeElement Tree & Mutation Command Batcher          |
+-------------------------------------------------------------------------+
                                    |
          (High-Performance Batch Buffer over JNI / C-FFI Boundary)
                                    v
+-------------------------------------------------------------------------+
|                        Rust Host Core Engine                            |
|                                                                         |
|  +------------------------------+     +------------------------------+  |
|  |    angnative-host-core       |     |     angnative-host-layout    |  |
|  |  Shadow Tree Management      |     |  Taffy 0.5.2 Layout Adapter  |  |
|  |  Mutation Protocol & Diffs   |     |  Text Measurer & Flex Cache  |  |
|  +------------------------------+     +------------------------------+  |
|                                                                         |
|  +------------------------------+     +------------------------------+  |
|  |    angnative-host-engine     |     |     angnative-host-ffi       |  |
|  |  QuickJS Runtime Integration |     |  C-ABI & JNI Export Headers  |  |
|  |  Event Loop & Dispatcher     |     |  Zero-Copy Buffer Bridge     |  |
|  +------------------------------+     +------------------------------+  |
|                                                                         |
|  +------------------------------+     +------------------------------+  |
|  |    angnative-host-view       |     |   angnative-services-core    |  |
|  |  View Tree & Op Emitter      |     |  Diagnostics, Capability,    |  |
|  |  Platform Command Generation |     |  Crypto, State Sync Engine   |  |
|  +------------------------------+     +------------------------------+  |
+-------------------------------------------------------------------------+
                                    |
          +-------------------------+-------------------------+
          | Zero-Copy JNI                                     | Direct C-ABI
          v                                                   v
+-----------------------------+                     +-----------------------------+
|    Android Platform Shell   |                     |     iOS Platform Shell      |
|  (Kotlin / Material 3)      |                     |   (Swift / UIKit / Metal)   |
|  - JNI Native Bridge        |                     |   - xtool SwiftPM Harness   |
|  - Android View Hierarchy   |                     |   - Mach-O arm64 Runtime    |
|  - Spring & Bounce Physics  |                     |   - Direct View Mounting    |
+-----------------------------+                     +-----------------------------+
```

---

## 3. Subsystem Breakdown

### 3.1 Angular Layer & DOM-Free Renderer (`@angnative/core`)
Angular is uniquely suited for non-DOM environments because its core architecture strictly decouples change detection and dependency injection from rendering via the `Renderer2` and `RendererFactory2` SPI.

- **No `@angular/platform-browser`**: Browser-specific classes (`DomAdapter`, `DomRendererFactory2`, HTML sanitizer) are excluded, preventing DOM leakage.
- **`NativeElement`**: Lightweight in-memory virtual nodes identified by a 64-bit integer (`node_id`).
- **Mutation Batching**: When Angular completes a change detection pass, all `CreateNode`, `SetProps`, `SetText`, `InsertChild`, and `RemoveChild` operations are compressed into a single mutation buffer and flushed to Rust.

### 3.2 Rust Host Engine (`crates/angnative-host-*`)
The Rust engine serves as the single source of truth for the UI shadow tree and layout computation:

1. **`angnative-host-core`**:
   - Manages the canonical UI tree and node lifecycle.
   - Parses the mutation protocol with zero allocations.
   - Enforces element schema and property types.
2. **`angnative-host-layout`**:
   - Adapts [Taffy 0.5.2](https://github.com/DioxusLabs/taffy) for Flexbox and CSS Grid calculations.
   - Maintains an asynchronous text-measuring cache so complex text wraps without blocking the main UI thread.
3. **`angnative-host-ffi`**:
   - Exposes clean C-ABI functions defined in `angnative_host_ffi.h`.
   - On Android: provides fast JNI methods (`Java_com_angnative_app_JniBridge_*`).
   - On iOS / Desktop: links directly as a static library (`libangnative_host_ffi.a`).
4. **`angnative-host-engine`**:
   - Embeds QuickJS for standalone JavaScript execution loops and zoneless event dispatching.
5. **`angnative-services-core`**:
   - Capability-aware degradation: detects platform features (biometrics, crypto, haptics) and provides standard fallbacks if a platform lacks support.

### 3.3 Android Platform Shell (`platforms/android`)
- **Native Views**: Maps `<na-view>` to `ViewGroup` / `FrameLayout`, `<na-text>` to `TextView`, `<na-button>` to `MaterialButton`.
- **High-Speed JNI**: Dispatches layout bounds and receives touch events (`press`, `change`, `scroll`) in sub-millisecond roundtrips.
- **R8 / Proguard Optimization**: Minified Kotlin codebase with an aggressive size budget (< 15 MB standalone APK).

### 3.4 iOS Platform Shell (`platforms/ios`)
- **Direct C-ABI**: Swift interacts with `angnative_host_ffi.h` with zero JNI overhead.
- **Cross-Platform Compilation**: Built via `xtool` with Darwin SDK and Swift 6 toolchain directly on Linux/WSL without requiring macOS hardware for compilation.
- **Native UIKit Rendering**: Maps nodes to `UIView`, `UILabel`, `UIButton`, and `UIScrollView`.

---

## 4. Concurrency & Threading Model

```
+------------------+         +-------------------+         +------------------+
| Angular/JS Thread|         |  Rust Host Worker |         | Platform UI Main |
+------------------+         +-------------------+         +------------------+
         |                             |                            |
         |-- Flush Mutation Batch ---->|                            |
         |                             |-- Taffy Layout Compute --->|
         |                             |<-- Text Measurement -------|
         |                             |                            |
         |                             |-- Emit Platform Ops ------>|
         |                             |                            |-- Mount Views
         |                             |                            |-- Render Frame (60-120fps)
         |<--------- Dispatch Event (Touch/Click) ------------------|
```

1. **Angular / JS Thread**: Runs change detection, updates signals, and serializes mutation batches.
2. **Rust Background Worker**: Computes layout geometry, diffs shadow trees, and emits minimum draw operations.
3. **Platform UI Thread**: Pure rendering and direct user touch handling. Never blocked by JavaScript execution.

---

## 5. Security & Isolation Model

- **No Web Inspector vulnerabilities**: Since there is no browser environment or WebView, DOM injection (XSS) attacks are eliminated.
- **Memory Safety**: The core engine is written in safe Rust with strictly validated FFI boundaries.
- **Sandbox Compliance**: Compatible with Android SELinux policies and iOS App Sandbox requirements.
