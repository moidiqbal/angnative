# Evidence — Final socket-only MarketPulse 9-test rerun (physical OnePlus)

**Date:** 2026-11-27
**Repo / worktree:** `C:\Users\Moid Iqbal\Projects\angnative` (isolated worktree
`C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing`)
**Branch / HEAD:** `feat/quickjs-bytecode-loader` at `e693061` after the local #5
implementation commit.
**Base artifact:** retained final socket-only APK from local `v0.3.0`
(`9f909408d29a242fbbe3ef4a8af37fd4747d09be`).
**Supersedes:** the physical-device caveat in
`docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md` §Result and the related tracker rows.
**Outcome:** **physical OnePlus `b6640c8f` passed the full nine-test generic
instrumentation suite: `OK (9 tests)`, exit `0`, 1.89 s.** No push.

## 1. Device attached

`adb` is not on PATH; the SDK copy was used directly.

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' devices -l
```

Observed:

```text
List of devices attached
b6640c8f               device product:CPH2551 model:CPH2551 device:OP5973L1 transport_id:7
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1
```

```powershell
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
& $adb -s b6640c8f shell getprop ro.product.model
& $adb -s b6640c8f shell getprop ro.build.version.release
& $adb -s b6640c8f shell getprop ro.product.cpu.abi
```

Observed:

```text
CPH2551
16
arm64-v8a
```

## 2. Artifacts and test APK preparation

The target app APK is the retained final socket-only build; the instrumentation
APK was built from the isolated worktree source. The app APK was not rebuilt.

```powershell
Get-FileHash -Algorithm SHA256 'dist\showcases\market-pulse\app-debug.apk' | Select-Object -ExpandProperty Hash
(Get-Item 'dist\showcases\market-pulse\app-debug.apk').Length
```

Observed:

```text
EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7
33149157
```

A first attempt to build the instrumentation APK failed because the Android
native library had not been staged in this worktree:

```powershell
$env:ANDROID_HOME='C:\Android\Sdk'; $env:ANDROID_SDK_ROOT='C:\Android\Sdk'; $env:JAVA_HOME='C:\Program Files\Java\jdk-25.0.5.1'; $env:PATH="C:\Android\Sdk\platform-tools;$env:PATH"; .\gradlew.bat assembleDebugAndroidTest --no-daemon
```

Observed failure excerpt:

```text
CMake Error at CMakeLists.txt:45 (message):
  No AngNative Rust library for arm64-v8a; run
  scripts/build-android-native.ps1
BUILD FAILED in 27s
```

The staged Rust libraries were built with the repository script:

```powershell
& .\scripts\build-android-native.ps1
```

Observed tail:

```text
   Compiling angnative-host-engine v0.1.0 (C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing\crates\angnative-host-engine)
   Compiling angnative-runtime-bridge v0.1.0 (C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing\crates\angnative-runtime-bridge)
    Finished `release` profile [optimized] target(s) in 17.96s
Android Rust libraries rebuilt (unified runtime bridge).
```

The instrumentation APK build then succeeded:

```powershell
$env:ANDROID_HOME='C:\Android\Sdk'; $env:ANDROID_SDK_ROOT='C:\Android\Sdk'; $env:JAVA_HOME='C:\Program Files\Java\jdk-25.0.5.1'; $env:PATH="C:\Android\Sdk\platform-tools;$env:PATH"; .\gradlew.bat assembleDebugAndroidTest --no-daemon
```

Observed tail:

```text
> Task :app:packageDebugAndroidTest
> Task :app:createDebugAndroidTestApkListingFileRedirect
> Task :app:assembleDebugAndroidTest

BUILD SUCCESSFUL in 31s
58 actionable tasks: 18 executed, 40 up-to-date
```

```powershell
$test = 'platforms\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk'
Get-FileHash -Algorithm SHA256 $test | Select-Object -ExpandProperty Hash
(Get-Item $test).Length
```

Observed:

```text
A88961CDE3F229E6160EE0F32AF0774DF3C7A728582426E16CD82847A084F7FD
410802
```

Gradle/native staging temporarily touched tracked asset and `jniLibs` files;
those were restored with `git checkout --` after the test APK was built. No
tracked source change was part of this rerun.

## 3. Install, instrument, and APK round trip

Command used (writes the retained raw log at each step):

```powershell
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$log = 'docs\assets\market-pulse-final-socket-9test-oneplus.log'
$app = 'dist\showcases\market-pulse\app-debug.apk'
$test = 'platforms\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk'
Remove-Item $log -ErrorAction SilentlyContinue
function W($m) { $m | Tee-Object -FilePath $log -Append }
W '=== DEVICE IDENTITY ==='
& $adb devices -l | Tee-Object -FilePath $log -Append
& $adb -s b6640c8f shell getprop ro.product.model | Tee-Object -FilePath $log -Append
& $adb -s b6640c8f shell getprop ro.build.version.release | Tee-Object -FilePath $log -Append
& $adb -s b6640c8f shell getprop ro.product.cpu.abi | Tee-Object -FilePath $log -Append
W '=== APK HASHES ==='
Get-FileHash -Algorithm SHA256 $app,$test | Format-Table -AutoSize | Out-String | Tee-Object -FilePath $log -Append
W '=== INSTALL APP ==='
& $adb -s b6640c8f install -r $app 2>&1 | Tee-Object -FilePath $log -Append
W "install_app_exit=$LASTEXITCODE"
W '=== INSTALL TEST ==='
& $adb -s b6640c8f install -r $test 2>&1 | Tee-Object -FilePath $log -Append
W "install_test_exit=$LASTEXITCODE"
W '=== INSTRUMENTATION ==='
& $adb -s b6640c8f shell am instrument -w -r com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner 2>&1 | Tee-Object -FilePath $log -Append
$code = $LASTEXITCODE
W "instrument_exit=$code"
exit $code
```

Observed install lines:

```text
Performing Streamed Install
Success
install_app_exit=0
Performing Streamed Install
Success
install_test_exit=0
```

Observed instrumentation summary (exit `0`):

```text
INSTRUMENTATION_STATUS: numtests=9
...
INSTRUMENTATION_RESULT: stream=

Time: 1.89

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

The installed app was pulled back from the device to prove it was the same APK:

```powershell
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$tmp = 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\oneplus-installed-base.apk'
$path = ((& $adb -s b6640c8f shell pm path com.angnative.app) -replace '^package:','').Trim()
Write-Output "device_path=$path"
& $adb -s b6640c8f pull $path $tmp
Get-FileHash -Algorithm SHA256 $tmp | Select-Object -ExpandProperty Hash
(Get-Item $tmp).Length
```

Observed:

```text
device_path=/data/app/~~cH7iaLeLTg22f_mZAC6TGQ==/com.angnative.app-wwO5z5D3fL3kKfcT42dHyA==/base.apk
/data/app/~~cH7iaLeLTg22f_mZAC6TGQ==/com.angnative.app-wwO5z5D3fL3kKfcT42dHyA==/base.apk: 1 file pulled, 0 skipped. 38.3 MB/s (33149157 bytes in 0.825s)
EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7
33149157
```

The complete raw log is retained at
`docs/assets/market-pulse-final-socket-9test-oneplus.log` (7,791 bytes, SHA-256
`7C4E1F573C3D64A7C2B957C3E54DB7313ADC2B6DD3AA9E3AA61C58F561AFEE60`).

## 4. Result

| Check | Result |
|---|---|
| Target app SHA-256 (local and pulled from device) | ✅ `EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7`, 33,149,157 B |
| Physical device | ✅ OnePlus Open `b6640c8f`, `CPH2551`, Android 16, `arm64-v8a` |
| Test APK SHA-256 | ✅ `A88961CDE3F229E6160EE0F32AF0774DF3C7A728582426E16CD82847A084F7FD`, 410,802 B |
| Full generic instrumentation suite | ✅ `OK (9 tests)`, exit 0, 1.89 s |
| Raw log retained | ✅ `docs/assets/market-pulse-final-socket-9test-oneplus.log` |
| Emulator result retained separately | ✅ `docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md` |
| iOS/Mac verification | ⚠️ still blocked (no macOS/Xcode) |

## 5. Scope limits

- This is an instrumentation rerun, not a new live-market-data capture. The
  socket-only feed evidence remains `docs/evidence/ANDROID-MARKETPULSE.md` §11.
- The suite proves instrumentation, lifecycle, tagged factory and thread/main-
  looper behavior; it makes no real-time or full-feature claim.
- The app APK was not rebuilt; the instrumentation APK was built from the
  isolated worktree source after staging the unified Rust library.
- No push, merge, tag change or main-repo Taffy change was made.

## 6. Diff hygiene after tracker updates

Command:

```powershell
git diff --check; git status --short
```

Observed (no whitespace errors):

```text
 M STATUS.md
 M docs/SHOWCASE-TRACKER.md
 M docs/UNSUPPORTED-FEATURES.md
 M docs/architecture/issue-register.md
 M docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md
?? docs/assets/market-pulse-final-socket-9test-oneplus.log
?? docs/evidence/FINAL-SOCKET-9TEST-ONEPLUS.md
```
