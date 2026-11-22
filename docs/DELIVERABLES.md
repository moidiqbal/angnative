# Phase 7 — Repository Deliverables Index

**Date:** 2026-11-21. Every link points to a file that exists in this repository.

## 1. Showcase source code
- `showcases/material/`, `showcases/primeng/`, `showcases/ngzorro/`, `showcases/taiga/`, `showcases/spartan/`
- `showcases/ecosystem-grid/`, `showcases/ecosystem-charts/`, `showcases/ecosystem-core/`
- `showcases/ecosystem-richtext/`, `showcases/ecosystem-calendar/`
- `packages/native-ui-core/` — shared reusable Android library

## 2. Independently installable debug APKs
`dist/showcases/<name>/app-debug.apk` for all eight showcase names above.
All were installed and launched on emulator API 34; `WebViews: 0` verified.

## 3. Native adapter implementations
- `packages/native-ui-core/android/.../NativeViewFactory.kt` — 8 PlatformOp callbacks, pluggable view creator
- `StyleCodec.kt`, `ViewTypes.kt`, `NativeUiTheme.kt`, `PlatformOp.kt`
- Adapter packages for individual libraries are intentionally not created (ADR 0004, no placeholders).

## 4. Compatibility matrix
- `docs/COMPATIBILITY.md` — native vs web adapter overview
- `docs/COMPATIBILITY-EVIDENCE.md` — Phase 4 per-feature evidence (versions, API level, backend, tests, limitations)
- `docs/ECOSYSTEM-COMPATIBILITY.md` — 11 ecosystem categories with fixed status vocabulary

## 5. Automated test suites
- `pnpm test` → 208 JS tests pass
- `cargo test --workspace` → 124 Rust tests pass (Windows: put Git's `patch.exe` first on PATH); `cargo test -p angnative-runtime-bridge -- --ignored` → 6 acceptance tests pass
- `packages/native-ui-core` Gradle `test` → 6 JUnit tests pass
- `scripts/verify-web.py` (Playwright) → web adapter render + click verified
- `scripts/verify-showcases.ps1` → APK install/launch/WebView checks
- `scripts/validate-build-tools.mjs` → 25 PASS / 0 FAIL / 6 INFO

## 6. Android build instructions
- `docs/ANDROID-BUILD.md` — SDK/NDK setup, two toolchain profiles (latest/legacy)
- `scripts/build-android-local.ps1`, `scripts/build-android-stack.ps1`, `scripts/build-android-native.ps1`
- `scripts/build-showcases.ps1`, `scripts/verify-showcases.ps1`

## 7. Benchmark report
- `docs/BENCHMARKS.md` + `docs/benchmarks/*.json` — initial single-sample startup/memory/APK size with caveats
- `scripts/measure-showcase.ps1` — reproducible collector

## 8. Unsupported-feature register
- `docs/UNSUPPORTED-FEATURES.md`

## 9. Architectural decisions
- `docs/adr/0001-taffy-0151-layout-adapter.md` (existing)
- `docs/adr/0002-zero-webview-showcases.md`
- `docs/adr/0003-native-reimplementation-labelling.md`
- `docs/adr/0004-shared-native-ui-core.md`

## 10. Future compatibility roadmap
- `docs/SHOWCASE-TRACKER.md` — phase-by-phase status
- `docs/ROADMAP.md` — product roadmap
- `docs/EVENT-BRIDGE.md` — Angular-listener round-trip remaining work

## Traceability
Community review → `docs/PROJECT-REVIEW-2026-11-20.md`; current truth → `STATUS.md`.
