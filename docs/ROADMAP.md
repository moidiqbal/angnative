# AngNative Engineering Roadmap

This roadmap breaks down the development of AngNative into concrete, orchestratable phases.

---

## Phase 1: Core Architecture & Native Layout Foundation (Current)
- [x] Monorepo workspace setup (Rust Cargo Workspace + TypeScript Packages).
- [x] System Architecture & Specification (`docs/ARCHITECTURE.md`).
- [ ] Implement `angnative-core` (Node tree, View mutations, serialization protocol).
- [ ] Implement `angnative-layout` (Taffy Flexbox bindings, node styling, layout computation).
- [ ] Implement `@angnative/core` (Angular custom `RendererFactory2` / `Renderer2`).
- [ ] Implement `@angnative/common` (Native primitive elements: `<native-view>`, `<native-text>`, `<native-button>`).

---

## Phase 2: Rust TypeScript Compiler & Embedded Runtime
- [ ] Implement `angnative-compiler` (Rust-based TypeScript parser and transformer via OXC/SWC).
- [ ] Implement `angnative-runtime` (QuickJS/Deno embedded JS runtime bridge).
- [ ] Bidirectional FFI bridge between Angular JS context and Rust Native ViewManager.
- [ ] Native event loop and asynchronous event dispatching.

---

## Phase 3: Desktop Shell (Windows, macOS, Linux)
- [ ] Implement `angnative-desktop` using `winit` for window lifecycle & input handling.
- [ ] WGPU rendering canvas or native widget mapping.
- [ ] Hot module reloading / live reload development harness for Desktop.

---

## Phase 4: Mobile Platform (Android NDK & Views)
- [ ] Implement `angnative-android` JNI bridge.
- [ ] Android Gradle harness & AAR build pipeline.
- [ ] Android Native ViewManager mapping layout geometry to `android.view.View`.
- [ ] Touch input, gestures, and lifecycle synchronization.

---

## Phase 5: Web Adapter & Demo Application
- [ ] Implement `@angnative/web` (HTML5/CSS fallback renderer).
- [ ] Create `apps/demo-app` demonstrating cross-platform counter, list, and form with Angular Signals.
- [ ] End-to-end integration tests across Desktop, Android, and Web.
