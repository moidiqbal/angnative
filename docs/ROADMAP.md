# 🗺️ AngNative Project Roadmap

**Status:** Active  
**License:** MIT

---

## Phase 0: Foundations & Architecture Spike ✅
- [x] Establish canonical identity, repository structure, and MIT licensing.
- [x] Prove custom Angular `Renderer2` feasibility without DOM (`@angnative/core`).
- [x] Validate Taffy layout engine integration in Rust (`angnative-host-layout`).
- [ ] Prototype zero-copy JNI bridge on Android (`platforms/android`). Current bridge uses JSON batches, not zero-copy.

---

## Phase 1: Core Engine & Android Shell ✅
- [x] Implement complete mutation protocol (`CreateNode`, `SetProps`, `SetText`, `InsertChild`, `RemoveChild`).
- [x] Build core UI primitive set (`<na-view>`, `<na-text>`, `<na-button>`, `<na-text-input>`, `<na-scroll-view>`, `<na-image>`, `<na-pressable>`).
- [x] Integrate QuickJS embedded JavaScript runtime (`angnative-host-engine`).
- [x] Wire the Android JNI bridge end-to-end: 8 Rust `PlatformOp` callbacks → real Android views, `free_string` on all returned strings, real JNI global-ref `user_data`, layout request built from the live snapshot, ops-driven demo UI, native press → Rust batch (`onNativePress`).
- [x] Build the Android shell on Windows: SDK/NDK 27.2 + CMake 3.22.1 + JDK 17 installed; Rust host libs rebuilt for 3 ABIs; `./gradlew assembleDebug` **BUILD SUCCESSFUL**; `app-debug.apk` (21.9 MB) contains `libangnative.so` + `libangnative_host_ffi.so` for arm64-v8a, armeabi-v7a and x86_64.
- [x] Install/launch the APK on an Android 34 x86_64 emulator and verify the click round trip visually: the demo renders from Rust `PlatformOp`s and "Click Me!" updates the Rust-hosted output node to "Hello World!" (screenshots in `docs/assets/android-demo*.png`).
- [x] Native event bridge scaffolding: FFI `set_event_callback` / `add|remove|dispatch_event` with tests, plus QuickJS-side `na.addEventListener` / `na.removeEventListener` / `dispatch_event` API (`docs/EVENT-BRIDGE.md`).
- [x] Full Angular listener round trip **in the embedded runtime**: `angnative-runtime-bridge` + real AOT bundle in QuickJS, 9 ignored acceptance tests pass (`docs/CODE-REVIEW-2026-11-22.md`).
- [ ] Load that bundle in a **shipping native shell on-device** and tap a real widget — still outstanding; shells remain imperative.
- [x] Enforce size budgets with `check-rust-size.mjs` and R8/Proguard optimizations.

---

## Phase 2: iOS Shell & Cross-Compilation 🟡
- [x] SwiftPM package with Swift 6 tools version and `AngNativeHost` / `AngNativeApp` / `AngNativeDemo` targets.
- [x] C ABI bridge in Swift (`SwiftFFI.swift`, UInt64 node ids, all returned strings freed).
- [x] UIKit view factory: all 8 `PlatformOp` callbacks → `UIView`/`UILabel`/`UIButton`/etc.
- [x] Demo command batch, Taffy layout application, and button press → Rust batch.
- [x] `scripts/build-ios-native.sh` builds the correct iOS Rust target and stages the static library for `swift build`.
- [ ] Run `./scripts/build-ios-native.sh simulator` and `swift test` on a Mac; verify UI and click in Simulator (no Mac in this environment).
- [ ] iOS touch/scroll coordinator refinements based on device feedback.

---

## Phase 3: Desktop & Developer Tooling 🟡
- [x] Implement CLI scaffolding (`@angnative/cli create`).
- [x] Implement dev server and doctor commands (`angnative doctor`).
- [x] Implement native desktop shell via `winit` and `wgpu` (`angnative-desktop`) — 11 tests pass; `cargo run` opens a live window titled "AngNative Desktop" (a WGSL dynamic-index validation panic was found during the first live run and fixed).
- [ ] Cross-platform Hot Module Replacement (HMR) daemon.
- [ ] Standalone component testing harness in Node.js without device emulator.

---

## Phase 4: Enterprise Services & Ecosystem 🟢
- [x] Offline data sync engine (`angnative-services-core`: queue, conflict detection/resolution, capability resolver).
- [ ] Durable SQLite/KV storage backend (current queue is in-memory).
- [ ] Hardware capability plugins (Camera, Biometrics, Geolocation, Push Notifications).
- [ ] High-level Material 3 & Cupertino component design system.
- [ ] Production showcase and public documentation portal.

---

## Phase 5: Web Adapter ✅
- [x] `@angnative/web` package with DOM `PlatformOp` backend.
- [x] `WebRenderer2` / `WebRendererFactory2` sharing the core command protocol.
- [x] Browser bootstrap (`bootstrapWebApp`) with zoneless default.
- [x] Re-export of canonical `na-*` primitives.
- [x] DOM render + click event integration tests (6 tests).
- [x] `pnpm start` serves a working browser demo — verified in headless Chromium via Playwright: no console errors, 11 `na-*` elements render, and clicking `+` updates `Count: 0 → 1` / `even → odd` (screenshots `docs/assets/web-demo*.png`).
