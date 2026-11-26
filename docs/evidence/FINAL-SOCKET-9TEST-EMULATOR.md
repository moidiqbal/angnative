# Evidence — Final socket-only MarketPulse 9-test rerun (emulator)

**Date:** 2026-11-27
**Repo:** `C:\Users\Moid Iqbal\Projects\angnative`
**Base:** local `v0.3.0` tag `9f909408d29a242fbbe3ef4a8af37fd4747d09be`
**Priority:** #3 after the web AOT migration and web size-budget gate.

## Claim under test

The **final socket-only** generic `platforms/android` APK now has a passing
rerun of the full nine-test instrumentation suite. This supersedes the earlier
caveat that only the historical CoinGecko APK had `OK (9 tests)`. The run below
is on the API 34 x86_64 emulator, **not** on the physical OnePlus `b6640c8f`;
no physical-device claim is added by this document.

---

## 1. Device availability

`adb` is not on PATH on this machine; the SDK copy was used directly.

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' devices -l
```

Observed before starting an emulator:

```text
List of devices attached

```

AVD inventory:

```powershell
& 'C:\Android\Sdk\emulator\emulator.exe' -list-avds
```

Observed:

```text
Android17
AngNative34
```

So no physical device was attached, but the existing `AngNative34` AVD was
available.

## 2. Artifacts installed

```powershell
Get-FileHash -Algorithm SHA256 'platforms\android\app\build\outputs\apk\debug\app-debug.apk','dist\showcases\market-pulse\app-debug.apk' | Format-Table -AutoSize
```

Observed:

```text
SHA256    EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7  platforms\android\app\build\outputs\apk\debug\app-debug.apk
SHA256    EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7  dist\showcases\market-pulse\app-debug.apk
```

The app APK and the checked-in `app-debug-androidTest.apk` in
`platforms/android/app/build/outputs/apk/androidTest/debug/` came from the same
build tree; no APK rebuild was performed for this rerun.

## 3. Emulator start, install and full suite

Reproducible core commands (the wait loops are omitted here; the exact terminal
session produced the observed lines below):

```powershell
$sdk = 'C:\Android\Sdk'
$adb = Join-Path $sdk 'platform-tools\adb.exe'
$emu = Join-Path $sdk 'emulator\emulator.exe'
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-25.0.5.1'
$env:PATH = "$sdk\platform-tools;$env:PATH"

Start-Process -FilePath $emu -ArgumentList @(
  '-avd','AngNative34','-no-snapshot','-no-boot-anim',
  '-gpu','swiftshader_indirect','-port','5554'
) -PassThru

& $adb install -r 'dist\showcases\market-pulse\app-debug.apk'
& $adb install -r 'platforms\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk'
& $adb shell am instrument -w -r com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner
```

Observed startup/install lines:

```text
emulator_pid=45512
waiting_for_device=0s
boot_completed=1
Performing Streamed Install
Success
Performing Streamed Install
Success
```

Observed instrumentation summary (exit `0`):

```text
INSTRUMENTATION_STATUS: numtests=9
...
INSTRUMENTATION_RESULT: stream=

Time: 4.77

OK (9 tests)


INSTRUMENTATION_CODE: -1
instrument_exit=0
```

The nine tests, in observed order, were:

1. `AngularCounterLayoutTest.angularCounterViewsDoNotOverlap`
2. `CandlestickViewFactoryTest.taggedCandlestickCreatesNativeViewAndParsesBarsString`
3. `RuntimeBridgeLifecycleTest.lifecycleWrappersTrackSuspendResumeAndStop`
4. `RuntimeBridgeLifecycleTest.staleU64HandleReturnsSafeSentinels`
5. `RuntimeBridgeLifecycleTest.taggedCustomPrimitivesMaterializeWithCanonicalTags`
6. `RuntimeBridgeThreadAffinityTest.wrongThreadDestroyIsANoOpAndDoesNotAbortTheProcess`
7. `RuntimeBridgeThreadAffinityTest.wrongThreadDispatchLoadAndGetHostReturnSentinelsAndBridgeStaysAlive`
8. `ViewFactoryMainThreadTest.dispatchNativeEventFromWorkerRunsOnMainAndReachesAngularListener`
9. `ViewFactoryMainThreadTest.viewFactoryCallbacksFromWorkerEntryAreMarshalledToMainLooper`

Every test reported `INSTRUMENTATION_STATUS_CODE: 1` then `0` (start then pass).
The complete raw log is retained at
`docs/assets/market-pulse-final-socket-9test-emulator.log` (6,473 bytes, SHA-256
`5480451E50F0D8374305A300B6F256CCC5F8003D1FB0E60939125ECBE41FA7AE`).

## 4. Device identity after boot

```powershell
& $adb devices -l
& $adb shell getprop ro.product.model
& $adb shell getprop ro.build.version.release
& $adb shell getprop ro.product.cpu.abi
& $adb shell pm path com.angnative.app
```

Observed:

```text
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1

sdk_gphone64_x86_64
14
x86_64
package:/data/app/~~ymdS6J4JQvntexssyXHj6A==/com.angnative.app-J4jXSu_MyNgrvflpAyZxpg==/base.apk
```

---

## Result

| Check | Result |
|---|---|
| Final socket APK SHA-256 | ✅ `EDD184C1…B387B7`, same as `dist/showcases/market-pulse/app-debug.apk` |
| Emulator API / ABI | ✅ Android 14, x86_64 (`sdk_gphone64_x86_64`) |
| Full generic instrumentation suite | ✅ `OK (9 tests)`, exit 0, 4.77 s |
| Raw log retained | ✅ `docs/assets/market-pulse-final-socket-9test-emulator.log` |
| Physical OnePlus rerun | ⚠️ not performed; no device attached |

## Scope limits

- This is an **emulator** result, not a physical OnePlus `b6640c8f` result.
- The suite proves instrumentation/lifecycle/factory/thread behavior; it is not
  a new live-market-data capture. The physical socket feed evidence remains
  `docs/evidence/ANDROID-MARKETPULSE.md` section 11.
- No APK or native library was rebuilt in this round.
- No push was performed.
