# Angular counter layout — physical OnePlus verification

**Date:** 2026-11-23
**Device:** OnePlus Open `b6640c8f`, model `CPH2551`, Android 16, `arm64-v8a`.
This is the physical device, not the emulator. No WebView is involved.

## Preparation

The staged JNI staticlibs were stale (missing `angnative_runtime_bridge_*`
symbols), so they were rebuilt from current source before the device run:

```text
pwsh -File scripts\build-android-native.ps1
```

The tracked legacy-named `showcases/taiga/.../libangnative_host_ffi.a` copies
touched by that script were reverted; no unrelated showcase files are part of
this change.

## Command

From `showcases/ecosystem-core`:

```powershell
$env:ANDROID_SERIAL='b6640c8f'
.\gradlew.bat connectedDebugAndroidTest --no-daemon
```

Gradle selected only the physical device: `Running tests on devices: CPH2551 - 16`.
Result: **8 tests, 0 failures, 0 errors, 0 skipped**.

| Suite | Tests | Result |
| --- | --- | --- |
| `AngularCounterLayoutTest` | 1 | passed — `angularCounterViewsDoNotOverlap` |
| `RuntimeBridgeLifecycleTest` | 3 | passed |
| `RuntimeBridgeThreadAffinityTest` | 2 | passed |
| `ViewFactoryMainThreadTest` | 2 | passed |

## What the first device run exposed — and what was fixed

1. **Static Angular text nodes get 0×0 frames on Android.** The first run found
   the buttons but their labels were empty: generated `ɵɵtext` children had
   zero-size layout rects. The demo now binds text through the supported
   `text` input on `na-text`/`na-button`, so the label lives on the styled
   element. The Rust host regression now accepts a label on the button node.
2. **Stale lifecycle test.** `lifecycleWrappersTrackSuspendResumeAndStop`
   called the raw `nativeRuntimeBridgeCreate()`, which does not populate
   `JniBridge.runtimeBridgeHandle`; the wrappers correctly returned `-1`.
   The test now uses `initHost()`/`destroyHost()`.
3. **Hardcoded node id.** `ViewFactoryMainThreadTest` dispatched to node `17`,
   which shifted when the AOT template changed. It now finds the `+` node from
   the host snapshot.

After these fixes the full suite above passed on the physical device.

## Live UI rects (screen pixels)

From `ui-before.xml`, scrolled to the Angular section. `uiautomator` renders the
U+2212 minus label as `-`.

| Control | Bounds `[left,top][right,bottom]` |
| --- | --- |
| `Counter` | `[235,1694][755,1791]` |
| `Count: 0` | `[235,1791][625,1882]` |
| `−` button | `[235,1921][391,2038]` |
| `reset` button | `[417,1921][573,2038]` |
| `+` button | `[599,1921][755,2038]` |
| `even` | `[235,2077][495,2142]` |

All rects have positive size and are pairwise non-overlapping. The button
origins increase left-to-right (`235 < 417 < 599`) and the counter label sits on
a different row. `logcat.txt` records `Angular layout applied=true`; the three
buttons are layout nodes 10/11/12 at parent-relative `x = 0/56/112`, each
`48×36` dp.

The worker-thread event dispatch test reached a live Angular listener; the
verified property is one delivery per event (canonical JS thread-local path + legacy FFI fallback).

## Retained artifacts

- `docs/evidence/angular-counter-oneplus/TEST-b6640c8f.xml`
- `docs/evidence/angular-counter-oneplus/html-report/`
- `docs/evidence/angular-counter-oneplus/ui-before.xml`
- `docs/evidence/angular-counter-oneplus/logcat.txt`
- `docs/evidence/angular-counter-oneplus/adb-devices.txt`
- `docs/assets/angular-counter-oneplus.png` — 2268×2440 valid PNG; the
  OnePlus foldable `screencap` warning prefix was stripped before saving.

## Honest scope

- Screenshot retained; no video was captured.
- This is a local device run, not a GitHub Actions run. `android-e2e` has still
  never executed on CI (upstream pnpm lockfile failure); no CI claim is made.
- No iOS/macOS runtime claim. The other nine shells and DOM compatibility docs
  were not changed.
