# 🗺️ AngNative Project Roadmap

**Status:** Active  
**License:** MIT

---

## Phase 0: Foundations & Architecture Spike ✅
- [x] Establish canonical identity, repository structure, and MIT licensing.
- [x] Prove custom Angular `Renderer2` feasibility without DOM (`@angnative/core`).
- [x] Validate Taffy layout engine integration in Rust (`angnative-host-layout`).
- [x] Prototype zero-copy JNI bridge on Android (`platforms/android`).

---

## Phase 1: Core Engine & Android Shell ✅
- [x] Implement complete mutation protocol (`CreateNode`, `SetProps`, `SetText`, `InsertChild`, `RemoveChild`).
- [x] Build core UI primitive set (`<na-view>`, `<na-text>`, `<na-button>`, `<na-text-input>`, `<na-scroll-view>`, `<na-image>`, `<na-pressable>`).
- [x] Integrate QuickJS embedded JavaScript runtime (`angnative-host-engine`).
- [x] Verify on Android 17 emulator with interactive event synchronization and bounce animations.
- [x] Enforce size budgets with `check-rust-size.mjs` and R8/Proguard optimizations.

---

## Phase 2: iOS Shell & Cross-Compilation 🟡
- [x] Install Darwin SDK and Swift 6 toolchain on Linux/WSL via `xtool`.
- [x] Build native Mach-O `arm64-apple-ios` static libraries and executable.
- [x] Package standalone iOS IPA (`TestApp.ipa`).
- [x] Verify physical iPhone 14 Plus USB pairing and developer mode handshake.
- [ ] Complete UIKit native view mapping (`UIView`, `UILabel`, `UIButton`).
- [ ] Implement iOS touch event responder and scroll coordinator.

---

## Phase 3: Desktop & Developer Tooling 🟡
- [x] Implement CLI scaffolding (`@angnative/cli create`).
- [x] Implement dev server and doctor commands (`angnative doctor`).
- [ ] Complete native desktop shell via `winit` and `wgpu` (`angnative-desktop`).
- [ ] Cross-platform Hot Module Replacement (HMR) daemon.
- [ ] Standalone component testing harness in Node.js without device emulator.

---

## Phase 4: Enterprise Services & Ecosystem 🟢
- [ ] Offline data sync and SQLite/KV storage engine.
- [ ] Hardware capability plugins (Camera, Biometrics, Geolocation, Push Notifications).
- [ ] High-level Material 3 & Cupertino component design system.
- [ ] Production showcase and public documentation portal.
