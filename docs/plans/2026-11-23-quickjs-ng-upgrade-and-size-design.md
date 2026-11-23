# QuickJS-ng upgrade + flagship APK size reduction — design

**Date:** 2026-11-23
**Status:** Implemented and measured (2026-11-23); evidence in
`docs/evidence/QUICKJS-NG-UPGRADE-AND-APK-SIZE.md`
**Scope:** `angnative-runtime-bridge` / `angnative-host-engine` Rust crates and the
flagship `showcases/ecosystem-core` Android shell. No engine bake-off, no iOS
claims, no ABI-strategy change.

## Decision

1. **Engine:** upgrade `rquickjs` from `0.5.1` to `0.14.0`. Its published
   `rquickjs-sys` crate vendors QuickJS-ng `0.16.2` (verified from
   `quickjs.h`: `QJS_VERSION_MAJOR 0 / MINOR 16 / PATCH 2`).
2. **Bundles:** flip esbuild `minify: false` → `true` for the shipped
   `quickjs-bundle.js` only. Feature bundles are test artifacts, not APK assets.
3. **Android release:** enable `isMinifyEnabled = true` and `isShrinkResources =
   true` with explicit keep rules for the JNI entry points.
4. **No bytecode and no lazy-load in this round.** `qjsc -b` bytecode needs a
   toolchain matched to the vendored engine and a runtime loader; lazy-load has
   no APK-size effect because feature bundles are not shipped.

## Verified baseline

- QuickJS-ng latest release is **v0.18.0** (published 2026-11-01, GitHub API),
  but no published Rust binding vendors it. `rquickjs` latest published crate is
  **0.14.0** (2026-09-18); its GitHub `master` submodule points at the v0.17.0
  commit, while the crates.io package still contains v0.16.2.
- Current repo uses `rquickjs 0.5.1` with QuickJS `VERSION = 2021-03-27`.
- Release universal 3-ABI APK measured **23,312,447 bytes** (22.23 MiB);
  uncompressed: classes 8,945,408 B, native libs 18,019,964 B (x86_64
  6,677,672; arm64-v8a 6,418,624; armeabi-v7a 4,923,668), assets 2,621,510 B,
  other 1,055,503 B, resources 573,777 B.
- Shipped asset `angnative-bundle.js` = **2,684,827 bytes**.
- Android release currently has `isMinifyEnabled = false`,
  `isShrinkResources = false`; Cargo release already has `strip = true`.
- OnePlus `b6640c8f` (CPH2551, Android 16, arm64-v8a): 8/8 instrumentation
  tests passed before this work.

## Implementation order

1. Change `Cargo.toml` to `rquickjs = { version = "0.14", features = ["classes",
   "chrono"] }`, run `cargo update -p rquickjs`, and fix compile/API breakage
   in the `JsEngine` implementation. Keep the engine-neutral trait intact.
2. Rebuild native Android archives with `scripts/build-android-native.ps1`
   (Android still needs `bindgen`). Revert untracked/generated taiga `.a`
   churn as before.
3. Set `minify: true` in `scripts/bundle-quickjs.mjs`, regenerate the bundle,
   and re-run the ignored Angular acceptance suite with the minified bundle.
4. Tighten `app/src/main/proguard-rules.pro` to keep `JniBridge` (native entries
   plus `onNative*` callbacks resolved via `GetMethodID`) and the launcher
   activity; enable R8 and resource shrinking in the release build type. Point
   `proguardFiles(...)` at the real file path.
5. Run the full gate: `cargo test --workspace`,
   `cargo test -p angnative-runtime-bridge -- --ignored`, `pnpm test`,
   `python scripts/check-ffi-declarations.py`.
6. Rebuild the release APK, measure bytes and breakdown, rerun
   `connectedDebugAndroidTest`, and report exact numbers. Commit/push only after
   every gate is green. If the APK is still above 20 MB, say so explicitly — no
   cap claim without the artifact. The OnePlus `b6640c8f` was detached at
   verification time, so the post-change device run used `emulator-5554`
   (API 34, x86_64): 8/8 tests passed, report retained under
   `docs/evidence/quickjs-ng-emulator-postchange/`. The post-change OnePlus
   result is **not verified**.

## Result (measured)

- All four gates green after the final edits: `cargo test --workspace`,
  `cargo test -p angnative-runtime-bridge -- --ignored` (15 passed / 0 failed),
  `pnpm test` (16 files / 240 tests),
  `python scripts/check-ffi-declarations.py` (OK).
- Android native archives rebuilt for all three ABIs with `bindgen`; Cargo.lock
  resolves all rquickjs crates at `0.14.0` and bindgen at `0.72.1`.
- Stack limit: 8 MiB in debug (unoptimized C frames), 2 MiB in release; release
  router acceptance test passed with 2 MiB.
- Shipped bundle: 1,098,643 bytes; R8 dex 727,496 bytes; resources 289,370 bytes.
- Three stale, unloaded `libangnative_host_ffi.so` files were removed from the
  flagship `jniLibs` (~2.4 MB saved) after confirming CMake links the static
  unified archive and Kotlin loads only `angnative`.
- `clean assembleRelease` succeeded. APK = **17,539,306 bytes** (17.539 MB /
  16.727 MiB), down from 23,312,447 bytes: **2,460,694 bytes under the decimal
  20,000,000-byte cap**. Full breakdown in the evidence doc above.

## Risks / limits

- rquickjs 0.5 → 0.14 is a multi-version API migration; Android bindgen and
  release `panic = "abort"` must both still build.
- R8 can break reflection/JNI callbacks; the eight device tests are the
  behavioral gate.
- No claim of exact v0.18.0, no head-to-head engine speed claim, no iOS
  runtime claim, and no statement that CI protects the device fix.
