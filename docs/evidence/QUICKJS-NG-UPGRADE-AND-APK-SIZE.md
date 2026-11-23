# QuickJS-ng upgrade + flagship APK size — measured evidence (2026-11-23)

Scope: `angnative-runtime-bridge` / `angnative-host-engine` plus the flagship
`showcases/ecosystem-core` Android shell. No WebView, no iOS/macOS runtime claims,
no head-to-head engine performance claim.

## Engine version actually shipped

- Workspace dependency is `rquickjs = "0.14"` (root `Cargo.toml`).
- Android target dependency in `crates/angnative-host-engine/Cargo.toml` is
  `rquickjs = { version = "0.14", features = ["bindgen", "classes", "chrono"] }`;
  bindgen is required because published `rquickjs-sys 0.14` ships no Android
  bindings.
- `Cargo.lock` resolves `rquickjs`, `rquickjs-core`, `rquickjs-macro`,
  `rquickjs-sys` all at `0.14.0`, with `bindgen 0.72.1`.
- The vendored engine is **QuickJS-ng 0.16.2** (`QJS_VERSION_MAJOR 0 / MINOR 16 /
  PATCH 2`). QuickJS-ng `v0.18.0` exists upstream but no published Rust binding
  vendors it, so the earlier ADR's "v0.18" target was not implementable this
  round. No claim of v0.18 is made.

## Stack-limit fix

QuickJS-ng uses substantially more native stack per JS frame than the previous
2021 snapshot. With the default 1 MiB limit, the router acceptance test failed
inside RxJS with `Maximum call stack size exceeded`.

`crates/angnative-host-engine/src/quickjs.rs` now sets a bounded stack limit:

- debug (unoptimized vendored C frames): **8 MiB**
- release (optimized frames): **2 MiB**

Evidence: `cargo test --release -p angnative-runtime-bridge -- --ignored
tests::angular_router_bootstraps_with_outlet` passed with the 2 MiB release limit.
The limit is never unlimited, so runaway scripts still fail gracefully. JS
exception capture in `eval()` was made actionable (`ctx.catch()` detail) during
root-cause analysis and retained.

## Full gate (after all final edits)

| Gate | Result |
|---|---|
| `cargo test --workspace` | all test binaries `ok`, 0 failed |
| `cargo test -p angnative-runtime-bridge -- --ignored` | **15 passed / 0 failed** (10 filtered) |
| `pnpm test` | **16 files / 240 tests passed** |
| `python scripts/check-ffi-declarations.py` | `FFI declaration check OK: 17 JNI methods, 22 Swift bindings (13 runtime bridge), 28 header declarations; iOS unified staticlib wiring OK` |

Release-mode router test above is additional evidence, not part of the four gates.

## Native rebuild and APK build

- `pwsh -File scripts/build-android-native.ps1` rebuilt the unified
  `libangnative_runtime_bridge.a` for `arm64-v8a`, `armeabi-v7a`, `x86_64` and
  staged it into the platform and showcase `jniLibs` directories.
- `scripts/bundle-quickjs.mjs` now sets `minify: true`; regenerated
  `dist/aot/quickjs-bundle.js` = **1,098,643 bytes** (previously 2,684,827).
  Gradle `preBuild` stages it as `assets/angnative-bundle.js`.
- Release build type enables `isMinifyEnabled = true` and
  `isShrinkResources = true`. The ProGuard file lives at
  `app/src/main/proguard-rules.pro`, so `proguardFiles(...)` now points there
  explicitly; keep rules cover `JniBridge` (native entries + `onNative*`
  callbacks resolved via `GetMethodID`) and the launcher activity.
- Three stale, unloaded `libangnative_host_ffi.so` files (17 Nov timestamps) were
  removed from the flagship `jniLibs`. CMake links the unified **static** archive
  and Kotlin loads only `angnative`, so they contributed ~2.4 MB without being
  used.
- `.\gradlew.bat clean assembleRelease --no-daemon` from
  `showcases/ecosystem-core`: **BUILD SUCCESSFUL**.

## Measured release APK (universal 3-ABI)

Path: `showcases/ecosystem-core/app/build/outputs/apk/release/app-release.apk`

| Measure | Value |
|---|---|
| APK bytes | **17,539,306** |
| Decimal MB | **17.539 MB** |
| MiB | **16.727 MiB** |
| Cap (decimal 20,000,000 bytes) | **met, 2,460,694 bytes headroom** |

Uncompressed breakdown (bytes):

| Category | After | Baseline (pre-change) |
|---|---:|---:|
| Native libs | 16,192,372 | 18,019,964 |
| `classes*.dex` | 727,496 | 8,945,408 |
| Assets | 1,100,024 | 2,621,510 |
| Other | 469,341 | 1,055,503 |
| Resources | 289,370 | 573,777 |

Baseline APK was 23,312,447 bytes (23.31 MB / 22.23 MiB). Reduction:
5,773,141 bytes (24.8% by bytes).

Packaged native libraries (uncompressed = stored):

| ABI | `libangnative.so` | `libc++_shared.so` |
|---|---:|---:|
| arm64-v8a | 4,481,160 | 1,292,904 |
| armeabi-v7a | 3,649,532 | 872,872 |
| x86_64 | 4,643,824 | 1,252,080 |

Staged bundle in APK: `assets/angnative-bundle.js` 1,098,643 bytes uncompressed,
316,419 bytes compressed.

## Device verification (post-change)

- `ANDROID_SERIAL=emulator-5554 .\gradlew.bat connectedDebugAndroidTest --no-daemon`
  from `showcases/ecosystem-core` → **BUILD SUCCESSFUL** on
  `AngNative34(AVD) - 14` (x86_64, API 34).
- Retained XML: `tests="8" failures="0" errors="0" skipped="0"`; all eight
  testcases passed. Artifacts:
  `docs/evidence/quickjs-ng-emulator-postchange/TEST-emulator-5554.xml`,
  `.../html-report/index.html`, `.../adb-devices.txt`.
- The physical OnePlus `b6640c8f` was **not attached** at verification time, so
  there is no post-change OnePlus result; its previously retained result is
  pre-change. This is recorded as **not verified** for the post-change build.
- Android JNI bridge runs on the main looper via `JniBridge.kt`
  `runOnMainForNative()`; no dedicated native JS thread and no explicit native
  stack size. Event semantics remain **one delivery per event (canonical JS
  thread-local path + legacy FFI fallback)**.

## Not verified / limits

- No retained AngNative performance artifact (cold start, jank, peak memory);
  no head-to-head superiority claim over V8/Hermes/NativeScript/RN/Flutter.
- No post-change OnePlus result: the device was detached; the earlier retained
  OnePlus result is pre-change. Emulator post-change result is retained above.
- GitHub Actions `android-e2e` was not run.
- Intrinsic text measurement is not wired; explicit demo sizing remains a
  workaround.
- QuickJS `v0.18.0` is upstream-only for this repo; no binding vendors it.
