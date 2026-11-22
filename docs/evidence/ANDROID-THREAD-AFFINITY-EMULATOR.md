# Android wrong-thread JNI verification (emulator)

**Date:** 2026-11-22

## Device

`adb devices -l` reported only:

```text
emulator-5554  device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa
```

Device properties:

```text
ro.product.model       = sdk_gphone64_x86_64
ro.build.version.release = 14
ro.product.cpu.abi     = x86_64
```

This is an Android 14 x86_64 emulator. It is **not** a physical-device result.

## What was verified

`showcases/ecosystem-core/app/src/androidTest/java/com/angnative/app/RuntimeBridgeThreadAffinityTest.kt`
makes real JNI calls from a `Thread` that is not the bridge owner thread:

- `nativeRuntimeBridgeDispatchEvent` → `-1` (`DISPATCH_PANIC` as `jint`)
- `nativeRuntimeBridgeLoadScript` → `2` (`LOAD_SCRIPT_PANIC`)
- `nativeRuntimeBridgeGetHost` → `0L` (`NULL`)
- owner-thread `nativeRuntimeBridgeLoadScript` still returns `0` afterwards
- `nativeRuntimeBridgeDestroy` from the worker thread is a no-op and the process survives

The Rust staticlib linked into the APK is a release build with
`[profile.release] panic = "abort"`, so reaching
`RuntimeBridge::assert_owner_thread()` on a wrong thread would abort the process.

## Commands

The first instrumentation run failed with `Test failed with status -1`. Root cause: the
prebuilt Android static libraries in `jniLibs` predated the C ABI guard commit
`fe8f08a`, so the APK still linked the old, unguarded archive. Rebuilding and staging
the current archive fixed the failure:

```powershell
pwsh -File scripts\build-android-native.ps1
cd showcases\ecosystem-core
.\gradlew.bat connectedDebugAndroidTest --no-daemon
```

## Result

```text
BUILD SUCCESSFUL in 24s
```

Instrumentation report: 2 tests, 0 failures, 0 skipped, 100% successful.

- `wrongThreadDestroyIsANoOpAndDoesNotAbortTheProcess` — passed
- `wrongThreadDispatchLoadAndGetHostReturnSentinelsAndBridgeStaysAlive` — passed

Retained report: `docs/evidence/android-thread-affinity-emulator.html`

## Honest scope

At the time of this run, `adb devices -l` showed no OnePlus Open; only
`emulator-5554` was attached, so this file claims only emulator verification.
The same instrumentation suite was later run on the physical OnePlus Open; see
`docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`.
