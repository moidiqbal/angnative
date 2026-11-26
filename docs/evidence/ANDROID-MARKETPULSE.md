# Android Evidence — MarketPulse combined demo (2026-11-25/26)

**Scope:** generic `platforms/android` shell, not the `ecosystem-core` clone and not a WebView app.
**Devices:** `emulator-5554`, API 34 x86_64 (`sdk_gphone64_x86_64`), AVD `AngNative34`; physical OnePlus `b6640c8f` (`CPH2551`), Android 16 / SDK 36, `arm64-v8a` (section 9).
**iOS / Mac:** not verified in this round.

This document records only commands that were run and output that was observed.
The demo is now a **crypto** demo: BTC/USD 30-minute OHLC from CoinGecko's public
endpoint. The earlier stock stage used Yahoo and returned HTTP 429 on-device;
that historical evidence is retained below but is no longer the active feed.
Animation claims are limited to deterministic `SetStyle` samples; no smooth
60 FPS, real-time easing or NativeScript/Flutter comparison is claimed.

---

## 1. Data source and honest label

The active feed is CoinGecko's public BTC/USD OHLC endpoint, probed from the host:

```powershell
curl.exe -sS -m 20 -w "`nHTTP=%{http_code}`n" "https://api.coingecko.com/api/v3/coins/bitcoin/ohlc?vs_currency=usd&days=1"
```

Observed: `HTTP=200`; rows are arrays
`[timestamp_ms, open, high, low, close]` spaced 1,800,000 ms (30 minutes).
Coinbase Exchange candles also returned `HTTP=200` but use a different column
order (`[time, low, high, open, close, volume]`), so CoinGecko was chosen for the
simplest honest mapping. The endpoint is public, unofficial and delayed; the UI
label is `CoinGecko public OHLC · 30m bars`. CoinGecko does not return volume,
so the normalized `v` field is explicitly `0` — no volume is fabricated.

On the emulator, this endpoint returned 48 bars and the UI showed
`BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars` (section 7). No real-time
claim is made.

Historical stock stage (retained for audit, not the active feed):

```powershell
curl.exe -L "https://query1.finance.yahoo.com/v8/finance/chart/AAPL?range=5d&interval=1d"
```

It returned HTTP 200 on the host but HTTP 429 on the device:

```text
11-25 17:29:27.642  4245  4271 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
```

That stage never fabricated bars. Its deterministic fixture pipeline was
verified separately in-process and by the Android factory test (sections 4 and 5).

---

## 2. Build and staged assets

Design: `docs/plans/2026-11-25-marketpulse-design.md`.

```powershell
pnpm compile:aot:marketpulse
pnpm bundle:marketpulse
.\gradlew.bat assembleDebug assembleDebugAndroidTest --no-daemon
```

Observed: AOT + bundle succeeded; the final crypto rebuild reported
`BUILD SUCCESSFUL in 35s`. The counter asset keeps its stable name because the
pre-existing instrumentation suite reads it; MarketPulse is a second asset
selected at runtime.

```powershell
Get-FileHash -Algorithm SHA256 dist\aot\market-pulse-bundle.js, platforms\android\app\src\main\assets\market-pulse-bundle.js, dist\aot\quickjs-bundle.js, platforms\android\app\src\main\assets\angnative-bundle.js | Format-Table -AutoSize
```

Observed hashes (dist and staged asset pairs match):

| Asset | SHA-256 |
|---|---|
| `market-pulse-bundle.js` | `46B2446D0E8783886F8B9CD902ED5F66E46BC9BF03A2945B229F144318E0FCB1` |
| `angnative-bundle.js` | `AD8C07D59ACE87A3A6E6E59FDBD060170FB5F8A2576A9B552B64A8B19E37A80F` |

APK staged for this evidence:

```powershell
Get-Item platforms\android\app\build\outputs\apk\debug\app-debug.apk | Select-Object FullName,Length,LastWriteTime
Get-FileHash -Algorithm SHA256 platforms\android\app\build\outputs\apk\debug\app-debug.apk
```

Observed: `app-debug.apk`, 33,064,229 bytes, SHA-256
`7FBE402075E58F6C489DC0138CEB5CFC5B8478E0D3716E2D231C3F5C0A13A0F2`.
Sections 3-5 below are historical white-screen/overlap-stage evidence; section 7
is the final crypto run.

---

## 3. Install, launch, screenshot, WebView and UI evidence

The emulator was (re)started with the repository script, then the app was
force-stopped and relaunched after a logcat clear so the evidence below comes
from one boot.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-android-emulator.ps1
& 'C:\Android\Sdk\platform-tools\adb.exe' install -r platforms\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk
& 'C:\Android\Sdk\platform-tools\adb.exe' logcat -c
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am force-stop com.angnative.app
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am start -n com.angnative.app/.MainActivity
```

Observed: `Success` for both installs, `Starting: Intent { cmp=com.angnative.app/.MainActivity }`.
During one capture the adb daemon dropped; `adb kill-server` followed by
`adb start-server` restored `emulator-5554 device`.

### 3.1 Filtered boot logcat

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' logcat -d -s AngNative:V MarketPulseFeed:V AndroidRuntime:E
```

Observed exactly:

```text
--------- beginning of main
11-25 17:29:26.695  4245  4245 I AngNative: AngNative runtime bridge created: 1
11-25 17:29:26.696  4245  4245 I AngNative: View factory registered for host: 0x78adbc3d2490 (ctx=0x78ae7c326580)
11-25 17:29:27.642  4245  4271 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
11-25 17:29:28.333  4245  4245 I AngNative: Angular boot complete. nodes=414 plusNode=null
```

`nodes=414` is an observed native shadow-tree node count from the pre-layout-fix
run. It is **not** a claim that 414 native views are the 10,000-row dataset; the
component generates 10,000 deterministic rows and renders a 200-row window
(section 4). This run was later diagnosed as zero-sized (section 3.5); the final
layout-fixed run reports `nodes=210`.

### 3.2 Zero WebView

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell dumpsys meminfo com.angnative.app | Select-String -Pattern 'WebViews|WebView|TOTAL|PID'
```

Observed:

```text
** MEMINFO in pid 4245 [com.angnative.app] **
                  Total    Dirty    Clean    Dirty    Total     Size    Alloc     Free
TOTAL    100018    88388     5964      192   191304    66421    46909    16000
TOTAL PSS:    100018            TOTAL RSS:   191304       TOTAL SWAP PSS:      192
Death Recipients:        0             WebViews:        0
```

The same check on the final layout-fixed process (`pid 4608`) reported
`WebViews:        0`, `TOTAL PSS: 81258`, `TOTAL RSS: 172940`.

### 3.3 UI hierarchy

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell uiautomator dump /data/local/tmp/marketpulse-window.xml
$xml = & 'C:\Android\Sdk\platform-tools\adb.exe' shell cat /data/local/tmp/marketpulse-window.xml | Out-String
[regex]::Matches($xml, 'content-desc="[^"]*"') | ForEach-Object { $_.Value } | Select-Object -Unique
[regex]::Matches($xml, 'text="[^"]*"') | ForEach-Object { $_.Value } | Select-Object -Unique
```

Observed:

```text
UI hierchary dumped to: /data/local/tmp/marketpulse-window.xml
content-desc=""
content-desc="Custom:na-candlestick"
text=""
text="Angular in Rust: nodes=414 rc=0 plusNode=null"
```

Retained pre-layout-fix artifacts: `docs/assets/market-pulse-ui.xml` and
`docs/assets/market-pulse-emulator.png` (44,000 bytes). These establish node
creation and `Custom:na-candlestick` presence, but the layout was zero-sized;
section 3.5 is the non-white verification.

### 3.4 Interactive refresh tap (second boot, same emulator)

The demo was relaunched with a cleared logcat, then the `Refresh` button was
tapped at coordinates derived from the live UI dump.

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' logcat -c
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am force-stop com.angnative.app
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am start -n com.angnative.app/.MainActivity
# wait 15s
& 'C:\Android\Sdk\platform-tools\adb.exe' shell uiautomator dump /data/local/tmp/marketpulse-demo-before.xml
```

Fresh UI dump showed the clickable `android.widget.Button` at
`bounds=[94,303][325,429]` and `android.view.View` with
`content-desc="Custom:na-candlestick"` at `bounds=[94,303][986,2217]`. Tap point
used: `(209,366)`.

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell input tap 209 366
```

Logcat after the tap:

```text
11-25 17:57:21.163  4188  4188 I AngNative: AngNative runtime bridge created: 1
11-25 17:57:21.163  4188  4188 I AngNative: View factory registered for host: 0x7932902e5220 (ctx=0x7931d0218180)
11-25 17:57:22.046  4188  4225 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
11-25 17:57:22.874  4188  4188 I AngNative: Angular boot complete. nodes=414 plusNode=null
11-25 17:58:01.922  4188  4188 I AngNative: dispatchNativeEvent(11, click) -> listeners invoked: 1
11-25 17:58:01.923  4188  4188 I AngNative: Angular tap node=11 -> 1 listener(s) refresh=true
11-25 17:58:02.886  4188  4225 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
```

The post-tap UI dump changed the status text to
`Angular in Rust: nodes=414 · public delayed feed`, and still contained
`content-desc="Custom:na-candlestick"`. This proves a real emulator tap reached
one live Angular listener and triggered the Kotlin refresh path. The refresh
also received HTTP 429; **no live bars were fabricated**.

Retained artifacts: `docs/assets/market-pulse-demo-before.xml`,
`docs/assets/market-pulse-demo-after.xml`, and
`docs/assets/market-pulse-demo-tapped.png` (44,005 bytes).

### 3.5 White-screen diagnosis and layout fix (supersedes 3.1–3.4 layout state)

The user reported `white screen show horahi..`. The node tree was not empty; a
temporary in-process snapshot count and a temporary Android layout log were
added, then reverted after the diagnosis.

In-process snapshot count from the pre-fix bundle:

```text
MARKETPULSE_SNAPSHOT_NODE_TYPES={"Button": 1, "Custom": 2, "Custom:na-candlestick": 1, "Text": 408, "View": 2}
```

Temporary Android log from the pre-fix APK:

```text
11-25 18:31:59.767  4942  4942 I AngNative: MARKETPULSE_LAYOUT_DBG rects=414 positive=0 zero=414 sample={"node_id":1,"x":0,"y":0,"width":0,"height":0}|{"node_id":2,"x":0,"y":0,"width":0,"height":0}|{"node_id":3,"x":0,"y":0,"width":0,"height":0}|{"node_id":4,"x":0,"y":0,"width":0,"height":0}|{"node_id":5,"x":0,"y":0,"width":0,"height":0}|{"node_id":6,"x":0,"y":0,"width":0,"height":0}|
```

Root cause: the MarketPulse AOT template had no `[style.*]` bindings, mirroring
the counter requirement in `apps/demo-app/src/counter.component.ts`; Rust layout
therefore produced 414 zero-size frames. The fix added explicit
`[style.flexDirection]`, `[style.width]`, `[style.height]`, `[style.padding]`,
`[style.gap]`, per-node dimensions and `[text]` bindings in
`apps/demo-app/src/marketpulse/main.marketpulse.ts`, then rebuilt the bundle and
APK.

```powershell
pnpm compile:aot:marketpulse
pnpm bundle:marketpulse
Get-FileHash -Algorithm SHA256 dist\aot\market-pulse-bundle.js
.\gradlew.bat assembleDebug assembleDebugAndroidTest --no-daemon
```

Observed: bundle hash changed from
`41CD8A447F1F3B5EB0495B049099413474318FB8A52908C5C655371337C10268` to
`A1B00352A792C5EAB7848F8F6115193C2B0F1B296C2F46C3CF13F1E856D8BAB2`; Gradle
`BUILD SUCCESSFUL`.

Diagnostic APK with the temporary layout log, after the bundle fix:

```text
11-25 18:36:32.571  4321  4321 I AngNative: MARKETPULSE_LAYOUT_DBG rects=210 positive=209 zero=1 sample={"node_id":1,"x":0,"y":0,"width":360,"height":3600}|{"node_id":2,"x":0,"y":0,"width":360,"height":3600}|{"node_id":3,"x":12,"y":12,"width":340,"height":18}|{"node_id":4,"x":12,"y":36,"width":340,"height":15}|{"node_id":5,"x":12,"y":57,"width":340,"height":28}|{"node_id":6,"x":0,"y":0,"width":330,"height":28}|
11-25 18:36:32.588  4321  4321 I AngNative: Angular boot complete. nodes=210 plusNode=null
```

One zero-size rect remains in the layout result; the other 209 rects have
positive width and height, and the final UI dump below confirms the visible
native views have non-zero bounds. The temporary log was removed and the final
APK rebuilt/reinstalled. Final boot with cleared logcat:

```text
11-25 18:38:43.225  4608  4608 I AngNative: AngNative runtime bridge created: 1
11-25 18:38:43.225  4608  4608 I AngNative: View factory registered for host: 0x7b16dcd9cc20 (ctx=0x7b18bccc8a80)
11-25 18:38:44.364  4608  4631 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
11-25 18:38:45.157  4608  4608 I AngNative: Angular boot complete. nodes=210 plusNode=null
```

Final UI dump parsing:

```text
total_nodes=50 textview_count=34
class=android.view.View desc=[Custom:na-candlestick] clickable=true bounds=[137,553][1029,933]
class=android.widget.Button desc=[] clickable=true bounds=[137,950][504,1028]
```

The 50 nodes are the visible subset reported by `uiautomator`; 34 are
`android.widget.TextView`. Visible text included
`MarketPulse — public delayed feed`,
`feed unavailable: IllegalStateException: HTTP 429 · 0 last-good bars`,
`AAPL `, and `OHLC 0 · seed42` … `OHLC 3 · seed42`. Final screenshot
`docs/assets/market-pulse-final.png` is 170,112 bytes at 1080×2400; a 2-pixel
grid sample counted 80,817 non-white samples. This is direct evidence that the
white-screen state is gone.

A real tap on the final layout used the button centre `(320,989)`:

```text
11-25 18:39:09.737  4608  4608 I AngNative: dispatchNativeEvent(8, click) -> listeners invoked: 1
11-25 18:39:09.737  4608  4608 I AngNative: Angular tap node=8 -> 1 listener(s) refresh=true
11-25 18:39:10.644  4608  4631 W AngNative: MarketPulse feed unavailable: IllegalStateException: HTTP 429
```

Retained artifacts: `docs/assets/market-pulse-layout-fixed.xml`,
`docs/assets/market-pulse-layout-fixed.png` (169,569 bytes, diagnostic APK),
`docs/assets/market-pulse-final.xml`, `docs/assets/market-pulse-final.png`
(170,112 bytes), `docs/assets/market-pulse-final-after.xml`, and
`docs/assets/market-pulse-final-tapped.png` (170,872 bytes).

---

## 4. In-process QuickJS acceptance test

```powershell
$env:PATH = "C:\Program Files\Git\usr\bin;$env:PATH"
cargo test -p angnative-runtime-bridge -- --ignored --exact tests::market_pulse_bundle_drives_candlestick_and_animation --nocapture
```

Observed:

```text
running 1 test
test tests::market_pulse_bundle_drives_candlestick_and_animation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 2.98s
```

This test asserts the `attr:bars` JSON-string mutation on the tagged
`na-candlestick` node, the 200-row window, and the deterministic animation
samples `0 → 0.5 → 1` plus `start,done` lifecycle events. It does **not**
prove real-time easing.

---

## 5. On-device factory test and full instrumentation suite

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am instrument -w -r -e class com.angnative.app.CandlestickViewFactoryTest com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner
```

Observed: `OK (1 test)` for `taggedCandlestickCreatesNativeViewAndParsesBarsString`.
This proves the tagged contract element creates a native Android view and parses
the shared OHLC bars string on-device. It is not a live-market-data assertion.

Full suite after the two-asset split:

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am instrument -w -r com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner
```

Observed:

```text
Time: 2.23

OK (9 tests)

INSTRUMENTATION_CODE: -1
```

The nine tests include `AngularCounterLayoutTest`,
`CandlestickViewFactoryTest`, `RuntimeBridgeLifecycleTest` (3),
`RuntimeBridgeThreadAffinityTest` (2), and `ViewFactoryMainThreadTest` (2).
The earlier two counter failures were caused by counter tests reading a
MarketPulse-overwritten asset; the two-asset split fixed that and this is the
post-fix result.

---

## 6. Final repository gates

```powershell
pnpm test
$env:PATH = "C:\Program Files\Git\usr\bin;$env:PATH"
cargo test --workspace
git diff --check
python scripts/check-ffi-declarations.py
```

Observed:

| Gate | Result |
|---|---|
| `pnpm test` | 16 files passed, **251 tests passed** |
| `cargo test --workspace` | **151 passed**, 0 failed, **16 ignored** (ignored tests include the AOT/QuickJS acceptance suite) |
| `git diff --check` | no whitespace errors; only LF→CRLF working-copy warnings |
| `python scripts/check-ffi-declarations.py` | `FFI declaration check OK: 34 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 28 header declarations; iOS unified staticlib wiring OK` |

The Windows `patch.exe` workaround is required for `rquickjs-sys`; the PATH line
above is part of the executed command.

---

## 7. Final crypto run: live bars, visible candles, same-textbox update

The final APK was installed with the crypto feed, logcat cleared, and the app
relaunched. The emulator was still `emulator-5554` (API 34 x86_64).

```powershell
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
& $adb install -r platforms\android\app\build\outputs\apk\debug\app-debug.apk
& $adb logcat -c
& $adb shell am force-stop com.angnative.app
Start-Sleep -Seconds 2
& $adb shell am start -n com.angnative.app/.MainActivity
Start-Sleep -Seconds 18
& $adb shell uiautomator dump /data/local/tmp/marketpulse-crypto-final-before.xml
& $adb exec-out screencap -p > docs\assets\market-pulse-crypto-boot.png
```

Observed boot log:

```text
11-25 19:24:51.590  4392  4392 I AngNative: AngNative runtime bridge created: 1
11-25 19:24:51.591  4392  4392 I AngNative: View factory registered for host: 0x7a6c4580f180 (ctx=0x7a6c15750ff0)
11-25 19:24:53.115  4392  4392 I AngNative: Angular boot complete. nodes=213 plusNode=null
```

Observed UI text from `docs/assets/market-pulse-crypto-boot.xml`:

```text
MarketPulse - BTC/USD OHLC
BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars
BTC/USD 58024
```

The tagged chart node is present:
`content-desc="Custom:na-candlestick" bounds="[137,556][1029,805]"`.
A 2-pixel `System.Drawing.Bitmap` sample inside those chart bounds counted
`green=927 red=1396 grid=2851`, so candles were actually painted, not merely
present as a node. `textview_count=36`; pairwise `TextView` bound comparison
found **0** intersections.

The `Demo bars` button centre is `(798,881)` in its `[588,826][1008,936]` bounds:

```powershell
& $adb shell input tap 798 881
Start-Sleep -Seconds 3
& $adb logcat -d -t 120 -s AngNative:V AndroidRuntime:E
& $adb shell uiautomator dump /data/local/tmp/marketpulse-crypto-final-after.xml
& $adb exec-out screencap -p > docs\assets\market-pulse-crypto-demo.png
```

Observed tap log:

```text
11-25 19:25:12.371  4392  4392 I AngNative: dispatchNativeEvent(11, click) -> listeners invoked: 0
11-25 19:25:12.415  4392  4392 I AngNative: dispatchNativeEvent(11, press) -> listeners invoked: 2
11-25 19:25:12.415  4392  4392 I AngNative: Angular tap node=11 action=demo-bars click=0 press=2 refresh=false
```

`na-pressable` listens for `press`; the dispatcher now sends both `click` and
`press` while walking native ancestors. Before/after comparison from the two UI
dumps:

| Item | Before tap | After tap |
|---|---|---|
| `TextView` count | 36 | 36 |
| Price text | `BTC/USD 58024` | `BTC/USD 60895` |
| Price bounds | `[137,456][1003,534]` | `[137,456][1003,534]` |
| Status | `BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars` | `BTC/USD · 32 bars · deterministic BTC/USD fixture (not live)` |
| Chart bounds | `[137,556][1029,805]` | `[137,556][1029,805]` |
| Chart green/red pixel samples | `927 / 1396` | `3813 / 0` |
| `TextView` bound intersections | 0 | 0 |

The price stayed in the same text box on device: the count and bounds did not
change, and the ignored QuickJS acceptance test separately asserts that the
price node **id** and total node count stay stable across
`__marketPulseApplyQuote` while its text changes `BTC/USD 13 → BTC/USD 99.5`.
The deterministic fixture is labelled `not live`; it is not presented as market
data. Retained artifacts: `docs/assets/market-pulse-crypto-boot.{png,xml}` and
`docs/assets/market-pulse-crypto-demo.{png,xml}`.

Final gates after this change: `pnpm test` 251 passed; `cargo test --workspace`
151 passed / 0 failed / 16 ignored; ignored MarketPulse QuickJS test 1 passed;
Android instrumentation `OK (9 tests)`; `python scripts/check-ffi-declarations.py`
OK; `git diff --check` no whitespace errors.

---

## 8. Signals + RxJS stream + delayed host polling (2026-11-25, second round)

Scope: add Angular Signals and an RxJS `Subject` stream to MarketPulse, plus
delayed host-side polling. No WebSocket/SSE or real-time claim. Design:
`docs/plans/2026-11-25-marketpulse-signals-rxjs-design.md`.

Commands run:

```powershell
pnpm compile:aot:marketpulse
pnpm bundle:marketpulse
cargo test -p angnative-runtime-bridge market_pulse_bundle_drives_candlestick_and_animation -- --ignored --nocapture
.\gradlew assembleDebug assembleDebugAndroidTest --no-daemon
& $adb install -r 'platforms\android\app\build\outputs\apk\debug\app-debug.apk'
& $adb logcat -c
& $adb shell am start -n com.angnative.app/.MainActivity
& $adb shell uiautomator dump /data/local/tmp/marketpulse-signals-poll.xml
& $adb exec-out screencap -p > docs\assets\market-pulse-signals-poll.png
& $adb shell am instrument -w -r -e package com.angnative.app com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner
python scripts/check-ffi-declarations.py
git diff --check
```

Observed:

- AOT compile completed and the bundle was written (`dist\aot\market-pulse-bundle.js`, 1.1 mb).
  `legalComments: 'none'` was added to `scripts/bundle-marketpulse.mjs` after the
  RxJS 6 tslib banner introduced trailing whitespace; the final `git diff --check`
  is clean.
- The ignored Rust MarketPulse acceptance test passed (`1 passed; 0 failed`). It
  now asserts `__marketPulseStreamEvents()` is `1` after the initial HttpClient
  response and `2` after a host push, checks the signal-backed status string, and
  keeps the stable price-node-id / unchanged-node-count assertions.
- `pnpm test`: 251 passed. `cargo test --workspace`: 151 passed / 0 failed /
  16 ignored. Android instrumentation: `OK (9 tests)`. FFI declaration check: OK.
- Emulator logcat with the default 30 s poll interval:

```text
11-25 20:00:50.747  4181  4181 I AngNative: Angular boot complete. nodes=213 plusNode=null
11-25 20:00:50.748  4181  4181 I AngNative: MarketPulse polling scheduled every 30000ms
11-25 20:01:20.911  4181  4181 I AngNative: MarketPulse poll pushed fresh quote: bars=48
11-25 20:01:50.861  4181  4181 I AngNative: MarketPulse poll pushed fresh quote: bars=48
11-25 20:02:20.909  4181  4181 I AngNative: MarketPulse poll pushed fresh quote: bars=48
```

- After three polls, `docs/assets/market-pulse-signals-poll.xml` showed
  `BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars`, price
  `BTC/USD 58182`, `TextViewCount=36`, `Custom:na-candlestick=True`, and chart
  bounds `[137,556][1029,805]`. A 2-pixel sample of that region in
  `docs/assets/market-pulse-signals-poll.png` counted `green=793 red=1601 grid=2147`,
  so the candles were still painted after reactive updates.
- Final rebuild with the clean bundle was reinstalled and booted:
  `Angular boot complete. nodes=213 plusNode=null` and
  `MarketPulse polling scheduled every 30000ms` at `20:05:37`.

Boundary: RxJS `interval` was deliberately not used because the QuickJS timer
shim (`crates/angnative-host-engine/src/quickjs_shims.rs:691-728`) makes
`setInterval` one-shot; an auto-repeating microtask chain would starve the
bounded host job loop. Polling is therefore Kotlin-scheduled and delayed, with
RxJS carrying the updates into Signals. No WebSocket/SSE exists in the source
or generated assets.

---

## 9. Physical OnePlus verification (2026-11-26)

The committed APK was re-installed and re-launched on the connected physical
device. Only the MarketPulse runtime, polling, candlestick painting and full
Android suite were re-run here; no new ABI, sustained-load, iOS/Mac, WebSocket/SSE
or source-change claim is made. No app source changed after commit
`1ba37c1b7ca0aa8349573d81480f391599585101`.

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' devices -l
& 'C:\Android\Sdk\platform-tools\adb.exe' shell getprop | Select-String -Pattern 'ro.build.version.release|ro.build.version.sdk|ro.product.cpu.abi|ro.product.model'
Get-FileHash -Algorithm SHA256 dist\showcases\market-pulse\app-debug.apk
& 'C:\Android\Sdk\platform-tools\adb.exe' install -r dist\showcases\market-pulse\app-debug.apk
```

Observed: `b6640c8f device product:CPH2551 model:CPH2551 device:OP5973L1 transport_id:1`;
`ro.build.version.release=16`, `ro.build.version.sdk=36`,
`ro.product.cpu.abi=arm64-v8a`, `ro.product.model=CPH2551`. The APK hash was
`3A648816480AFD6C7F040A95E27271D48E1E9956B0EB0093C8756A3B588D51ED`; install
reported `Performing Streamed Install` then `Success`.

Boot log recorded in `docs/assets/market-pulse-physical-boot.log`:

```text
11-26 14:24:15.504 31640 31640 I AngNative: AngNative runtime bridge created: 1
11-26 14:24:15.504 31640 31640 I AngNative: View factory registered for host: 0xb4000077aedee000 (ctx=0xb40000776aa46560)
11-26 14:24:16.415 31640 31640 I AngNative: Angular boot complete. nodes=213 plusNode=null
11-26 14:24:16.416 31640 31640 I AngNative: MarketPulse polling scheduled every 30000ms
```

Poll log recorded in `docs/assets/market-pulse-physical-poll.log`:

```text
11-26 14:24:46.425 31640 31689 W AngNative: MarketPulse crypto feed unavailable: UnknownHostException: Unable to resolve host "api.coingecko.com": No address associated with hostname
11-26 14:24:46.464 31640 31640 I AngNative: MarketPulse poll pushed fresh quote: bars=48
11-26 14:25:16.424 31640 31689 W AngNative: MarketPulse crypto feed unavailable: UnknownHostException: Unable to resolve host "api.coingecko.com": No address associated with hostname
11-26 14:25:16.474 31640 31640 I AngNative: MarketPulse poll pushed fresh quote: bars=48
```

The scheduler fired at the expected 30 s interval twice. Although the log line
says `poll pushed fresh quote`, the immediately preceding lines show DNS failure;
the honest claim is that the last-good cached payload was pushed into the RxJS
stream with `bars=48`. No fresh CoinGecko response was observed on the physical
device in this window, and no data was fabricated.

UI dump (`docs/assets/market-pulse-physical.xml`) observed:
`text="BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars"`, price
`BTC/USD 60838`, `TextViewCount=28`, `Custom:na-candlestick=True`, chart bounds
`[168,652][1064,960]`.

Screenshot capture required an explicit display ID. The default multi-display
capture emitted a warning prefix and was not used; `dumpsys SurfaceFlinger
--display-id` listed `4630946316123810436` (HWC display 3), which produced the
valid app image in `docs/assets/market-pulse-physical.png`:

```powershell
& 'C:\Android\Sdk\platform-tools\adb.exe' shell screencap -p -d 4630946316123810436 /data/local/tmp/marketpulse-physical.png
& 'C:\Android\Sdk\platform-tools\adb.exe' pull /data/local/tmp/marketpulse-physical.png docs\assets\market-pulse-physical.png
```

Observed: PNG `1116x2484`; a 2-pixel sample inside the chart bounds counted
`green=1476 red=883 grid=2834`, so candles were actually painted on the physical
screen. The status-text region counted `statusBright=2819`.

The Android test APK was rebuilt from the committed sources and re-installed:

```powershell
.\gradlew assembleDebugAndroidTest --no-daemon
& 'C:\Android\Sdk\platform-tools\adb.exe' install -r platforms\android\app\build\outputs\apk\androidTest\debug\app-debug-androidTest.apk
& 'C:\Android\Sdk\platform-tools\adb.exe' shell am instrument -w -r -e package com.angnative.app com.angnative.app.test/androidx.test.runner.AndroidJUnitRunner
```

Observed: `OK (9 tests)` on `b6640c8f`. During capture the screen was kept awake
with `settings put system screen_off_timeout 2147483647` and `svc power stayon
true`; after evidence capture both settings were restored to their observed
defaults (`30000` and `false`). This was a device-setting change only, not an app
change.

---

## 10. Boundaries of this evidence

- **Claimed:** APK builds/installs/launches; Angular AOT bundle boots in the Rust-owned QuickJS runtime; 213 native nodes on the final crypto boot; no WebView node in the UI dumps; `Custom:na-candlestick` in the UI hierarchy; native CandlestickView paints live BTC/USD bars (confirmed by green/red pixel samples); CoinGecko returned 48 real 30-minute OHLC bars on the emulator; a real `Demo bars` tap reaches two live Angular `press` listeners and updates the existing price text box in place; tagged factory parses the OHLC bars string on-device; deterministic animation samples `0 / 0.5 / 1` in-process; Angular Signals and an RxJS `Subject` drive the same native nodes; three delayed 30 s Kotlin polls delivered fresh 48-bar payloads without growing the 36 `TextView` nodes; the committed APK also booted (`nodes=213`), painted green/red candles and ran two 30 s scheduler pushes on physical OnePlus `b6640c8f`, with full instrumentation `OK (9 tests)` there.
- **Not claimed:** WebSocket/SSE, real-time or guaranteed feed delivery (the physical-device polls hit DNS failures and pushed last-good data), RxJS `interval`/JS timers as the polling source, CoinGecko volume data (`v` is explicitly 0), smooth/eased 60 FPS animation, 10,000 native rows (only the dataset has 10,000 rows), `RecyclerView` recycling, unchanged AG Grid/ECharts/GSAP compatibility, the earlier Yahoo stock feed working on-device (it returned HTTP 429), physical-device results beyond the OnePlus MarketPulse run in section 9 (ABI re-verification, sustained load, CPU/warm-start), iOS/Mac verification, or any NativeScript/Flutter head-to-head result.
