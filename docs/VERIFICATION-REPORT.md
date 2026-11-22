# AngNative — Full Verification Report

**Date:** 2026-11-21
**Machine:** Windows, `C:\Users\Moid Iqbal\Projects\angnative`

The "❌ Not tested — no pnpm/cargo" status list is from a different environment.
On this machine all toolchains are installed and every item below was actually executed.

## Verified results

| Item | Status | Evidence |
|---|---|---|
| Toolchain: pnpm / node / cargo / Android SDK | ✅ | pnpm 10.34.9, node v26.13.1, cargo 1.102.0-nightly, `C:\Android\Sdk` present |
| AOT compile | ✅ PASS | `pnpm run compile:aot` → "AOT compilation complete (Ivy definitions verified)"; `dist/aot/app.component.js` contains `ɵɵdefineComponent` |
| CLI production build | ✅ PASS | `node packages/cli/dist/cli.js build` → real ngc + esbuild → `dist/aot/bundle.js` |
| Rust build | ✅ PASS | `cargo build --workspace` → Finished, 0 errors |
| Rust tests | ✅ PASS | `cargo test --workspace` → **134 passed, 0 failed** (8 acceptance tests ignored by default) |
| Angular-in-QuickJS acceptance tests | ✅ PASS | `cargo test -p angnative-runtime-bridge -- --ignored` → **8 passed** (real AOT bundle boots, native click reaches Angular listener; forms/ngModel, router, animations, http and AOT i18n bundles load) |
| JS/TS unit tests | ✅ PASS | `pnpm test` → **220 passed** (16 files) |
| Desktop app | ✅ PASS | `cargo test --manifest-path crates/angnative-desktop/Cargo.toml` → 11 passed; window "AngNative Desktop" previously launched and click-tested |
| Android APK | ✅ PASS | **10 APKs** built under `dist/device-testing/` (19.2–19.3 MB each) |
| Android device run | ✅ PASS | all 10 installed/launched on **OnePlus Open (CPH2551, Android 16)**; `WebViews: 0` on every app; gfxinfo: grid 170 frames / 1.76% jank / 6 ms p50; valid screenshots in `docs/benchmarks/physical/`) |
| Size budget check | ✅ PASS | `node scripts/check-size-budget.mjs` → `GATE EXIT=0`, 203 B gzip / 60 KB budget |
| iOS app | ⚠️ NOT TESTED | Swift config + runtime-bridge bindings exist under `platforms/ios/`; `check-ffi-declarations.py` statically verifies 14 Swift bindings and unified staticlib wiring, but no macOS/Xcode here — `swift build`, `swift test`, Simulator and device runs intentionally skipped |

## Bottom line
All correctness blockers outside iOS/macOS are closed and pushed to `master`.
The flagship `ecosystem-core` shell loads a real AOT bundle and a native tap
reaches a live Angular listener, verified on emulator and OnePlus
(`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`). The other 9 showcases remain
imperative native-reimplementation concept demos by design (ADR 0003); remaining
items are explicit deferred features, not hidden gaps. The iOS shell is
code-complete but requires a Mac for `./scripts/build-ios-native.sh simulator`
and simulator verification — no iOS runtime claim is made here.

## Reproduce
```powershell
$env:PATH="C:\Program Files\Git\usr\bin;$env:PATH"   # Windows: avoid elevation-requiring patch.exe for rquickjs-sys
pnpm run compile:aot
node packages/cli/dist/cli.js build
cargo build --workspace
cargo test --workspace
pnpm test
node scripts/check-size-budget.mjs
```

