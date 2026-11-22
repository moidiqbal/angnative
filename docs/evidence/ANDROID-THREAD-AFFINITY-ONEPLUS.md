# Android wrong-thread JNI verification (physical OnePlus Open)

**Date:** 2026-11-22; updated 2026-11-23

## Device

`adb devices -l` after USB reconnection and RSA authorization:

```text
b6640c8f  device product:CPH2551 model:CPH2551 device:OP5973L1 transport_id:2
```

Device properties:

```text
ro.product.model         = CPH2551
ro.build.version.release = 16
ro.product.cpu.abi       = arm64-v8a
```

This is the physical OnePlus Open, not an emulator.

## Command

From `showcases/ecosystem-core`, with the ARM64 Rust staticlib already rebuilt and
staged by `pwsh -File scripts\build-android-native.ps1`:

```powershell
$env:ANDROID_SERIAL = 'b6640c8f'
.\gradlew.bat connectedDebugAndroidTest --no-daemon
```

Gradle selected only the physical device:

```text
Running tests on devices: CPH2551 - 16
BUILD SUCCESSFUL in 56s
```

## Result

Instrumentation report: 2 tests, 0 failures, 0 skipped, 100% successful.

- `wrongThreadDestroyIsANoOpAndDoesNotAbortTheProcess` — passed (0.004s)
- `wrongThreadDispatchLoadAndGetHostReturnSentinelsAndBridgeStaysAlive` — passed (0.006s)

The test makes real JNI calls from a non-owner `Thread` against a release Rust
staticlib (`panic = "abort"`). The wrong-thread calls returned the documented
sentinels (`-1`, `2`, `NULL`), the owner-thread bridge stayed usable, and the
wrong-thread `destroy` did not abort the process.

Retained report: `docs/evidence/android-thread-affinity-oneplus.html`

## Scope

This verifies the C ABI owner-thread guard on the physical device. UI-thread
marshalling of Rust→Kotlin ViewFactory callbacks and the real native tap → live
Angular listener were verified later on 2026-11-23; see
`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md` and `docs/EVENT-BRIDGE.md`.
