# Android UI-thread ViewFactory marshalling + live tap verification

**Date:** 2026-11-23
**Design:** `docs/plans/2026-11-23-android-ui-thread-marshalling-design.md`

## What was verified

1. Rust→Kotlin ViewFactory callbacks invoked from a worker-thread native entry
   point execute on `Looper.getMainLooper()`.
2. A real native tap reaches a live Angular listener and updates the native UI.
3. Both checks were run on the Android emulator and on the physical OnePlus Open.

## Build / setup

```powershell
pwsh -File scripts\build-android-native.ps1
```

The rebuild matters: a stale `jniLibs` archive caused an earlier wrong-thread
JNI test failure in this repo.

Instrumentation command (run from `showcases/ecosystem-core`):

```powershell
$env:ANDROID_SERIAL='emulator-5554'
.\gradlew.bat connectedDebugAndroidTest --no-daemon

$env:ANDROID_SERIAL='b6640c8f'
.\gradlew.bat connectedDebugAndroidTest --no-daemon
```

## Devices

| Device | Serial | Model | Android | ABI |
|---|---|---|---|---|
| Emulator | `emulator-5554` | `sdk_gphone64_x86_64` | 14 | `x86_64` |
| Physical OnePlus Open | `b6640c8f` | `CPH2551` | 16 | `arm64-v8a` |

## Instrumentation result

Full suite on each device: **4 tests, 0 failures, 0 skipped, 100% successful**.

- `RuntimeBridgeThreadAffinityTest` — 2 tests (wrong-thread JNI guard, retained
  from the earlier change).
- `ViewFactoryMainThreadTest` — 2 new tests:
  - `viewFactoryCallbacksFromWorkerEntryAreMarshalledToMainLooper`
  - `dispatchNativeEventFromWorkerRunsOnMainAndReachesAngularListener`

The new tests create/init the bridge with `runOnMainSync` so Rust's
`owner_thread` is main, install a `RecordingViewSink`, then call native entry
points from a real worker thread. The sink records `Looper.myLooper()` and
`Thread.currentThread().name` **inside each callback**. Every recorded looper
was `Looper.getMainLooper()`; no callback ran on `angnative-viewfactory-worker`.

Asserted callbacks are the ones the real native paths emit for the test inputs:
`onNativeCreateView`, `onNativeSetProps`, `onNativeSetText`,
`onNativeInsertChild`, `onNativeRemoveChild`. `SetFrame` is applied from Kotlin
layout code, while `Append` and `Destroy` are not emitted by these commands;
the tests do not claim those three were observed.

Retained HTML reports:

- `docs/evidence/android-viewfactory-mainthread-emulator.html`
- `docs/evidence/android-viewfactory-mainthread-oneplus.html`
- Full report directories: `docs/evidence/android-viewfactory-mainthread-emulator/`
  and `docs/evidence/android-viewfactory-mainthread-oneplus/`

## Real tap proof

App installed and launched with the explicit component name (the Kotlin package
is `com.angnative.app`, while `applicationId` is
`com.angnative.demo.ecosystem.core`):

```text
com.angnative.demo.ecosystem.core/com.angnative.app.EcosystemCoreActivity
```

For each device:

```text
adb shell uiautomator dump /sdcard/window.xml
python scripts/android-tap-coords.py <before.xml>
adb logcat -c
adb shell input tap <coords>
adb logcat -d -s AngNative:I
adb shell uiautomator dump /sdcard/after.xml
adb shell screencap -p /sdcard/screen.png
```

On both devices logcat contained:

```text
I AngNative: dispatchNativeEvent(18, click) -> listeners invoked: 0
I AngNative: Angular tap node=18 -> 0 listener(s)
I AngNative: dispatchNativeEvent(17, click) -> listeners invoked: 1
I AngNative: Angular tap node=17 -> 1 listener(s)
```

The `18 → 0` hop is the ancestor walk; `17 → 1` is the Angular listener. The
after dump on both devices contained `text="Count: 1"` (before: `Count: 0`).

Retained artifacts:

| Artifact | Emulator | OnePlus |
|---|---|---|
| Before XML | `docs/evidence/tap-emulator-before.xml` | `docs/evidence/tap-oneplus-before.xml` |
| Logcat | `docs/evidence/tap-emulator-logcat.txt` | `docs/evidence/tap-oneplus-logcat.txt` |
| After XML | `docs/evidence/tap-emulator-after.xml` | `docs/evidence/tap-oneplus-after.xml` |
| Screenshot | `docs/assets/angular-real-tap-emulator-count1.png` | **Not retained** — device disconnected during capture; the attempted `angular-real-tap-oneplus-count1.png` was 0 bytes and is not evidence |

## Honest scope

- No WebView is involved; the proof uses the real AOT bundle in the Rust-owned
  QuickJS runtime and real Android views.
- The event bridge still has a canonical JS thread-local path and a legacy FFI
  fallback. The verified property is **one delivery per event**, not a single
  code path.
- iOS was not run: no Mac/Xcode, Swift compile, Simulator, or device evidence.
