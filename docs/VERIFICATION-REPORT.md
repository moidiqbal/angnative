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
| Rust tests | ✅ PASS | `cargo test --workspace` → **129 passed, 0 failed** (6 acceptance tests ignored by default) |
| Angular-in-QuickJS acceptance tests | ✅ PASS | `cargo test -p angnative-runtime-bridge -- --ignored` → **6 passed** (real AOT bundle boots, native click reaches Angular listener; forms, router, animations, http bundles load) |
| JS/TS unit tests | ✅ PASS | `pnpm test` → **219 passed** (16 files) |
| Desktop app | ✅ PASS | `cargo test --manifest-path crates/angnative-desktop/Cargo.toml` → 11 passed; window "AngNative Desktop" previously launched and click-tested |
| Android APK | ✅ PASS | **10 APKs** built under `dist/device-testing/` (19.2–19.3 MB each) |
| Android device run | ✅ PASS | all 10 installed/launched on **OnePlus Open (CPH2551, Android 16)**; `WebViews: 0` on every app; gfxinfo: grid 170 frames / 1.76% jank / 6 ms p50; valid screenshots in `docs/benchmarks/physical/`) |
| Size budget check | ✅ PASS | `node scripts/check-size-budget.mjs` → `GATE EXIT=0`, 203 B gzip / 60 KB budget |
| iOS app | ⚠️ NOT TESTED | 6 Swift/config files exist under `platforms/ios/`; no macOS/Xcode here — build intentionally skipped |

## Bottom line
Everything except iOS/macOS is implemented, built, tested and pushed to `master`
(latest commit `fc3370c`). The iOS shell is code-complete but requires a Mac for
`./scripts/build-ios-native.sh simulator` and simulator verification. The embedded
Angular runtime is verified in-process; loading the bundle in a shipping native
shell on-device remains the integration milestone (see `CODE-REVIEW-2026-11-22.md`).

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

