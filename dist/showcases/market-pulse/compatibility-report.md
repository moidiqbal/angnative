# MarketPulse — compatibility / verification report (2026-11-26, socket-only active build)

## Identity

| Field | Value |
|---|---|
| Showcase | MarketPulse — combined heavy windowed OHLC + Angular animations + native match-derived candlestick |
| APK | `dist/showcases/market-pulse/app-debug.apk` (copied from `platforms/android/app/build/outputs/apk/debug/app-debug.apk`, 33,149,157 bytes) |
| APK SHA-256 | `EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7` |
| Package ID | `com.angnative.app` (generic `platforms/android` shell, not a separate showcase package) |
| Build profile | latest: Gradle 9.9.2 / AGP 9.5.2 / JDK 25 |
| Device | physical OnePlus `b6640c8f` (`CPH2551`), Android 16 / API 36, `arm64-v8a`, 1116×2484 |
| Evidence | `docs/evidence/ANDROID-MARKETPULSE.md` section 11; `docs/evidence/ANDROID-SUSTAINED-LOAD.md`; plans `docs/plans/2026-11-25-marketpulse-design.md` and `docs/plans/2026-11-25-marketpulse-signals-rxjs-design.md` |

Superseded APK hashes: `3A648816480AFD6C7F040A95E27271D48E1E9956B0EB0093C8756A3B588D51ED`
(CoinGecko stage) and `0EB26AD158355EBC2D131AA2ECCE7DAD5468565DB38757B3933EA4F817AA2DEC`
(socket build before the subscription-format fix). They are not the active build.

## What was actually built

- Real Angular AOT bundle `dist/aot/market-pulse-bundle.js` (SHA-256
  `F960CBD7659AF04D4012B1F57A14C36D582591E1E2FB8F459EDBCA2B0A35F3C2`)
  executed by the Rust-owned QuickJS runtime. No WebView. The template uses
  explicit `[style.*]`/`[text]` bindings, so Rust layout produces non-zero
  frames.
- Two assets are staged: `assets/angnative-bundle.js` keeps the counter app and
  its instrumentation assertions (SHA-256
  `AD8C07D59ACE87A3A6E6E59FDBD060170FB5F8A2576A9B552B64A8B19E37A80F`), and
  `assets/market-pulse-bundle.js` is selected by the Android shell at runtime.
  Dist/staged hashes match for both.
- Chart is the contract element `na-candlestick` (`Custom:na-candlestick`),
  created by `NativeViewFactory` and drawn with Android `Canvas`. OHLC bars cross
  the bridge as a JSON string on `attr:bars`.
- **Socket-only market data:** the Kotlin host opens
  `wss://ws-feed.exchange.coinbase.com` and subscribes with exactly
  `{"type":"subscribe","product_ids":["BTC-USD"],"channels":["matches"]}`.
  Object-form channels were rejected by the endpoint (`Failed to subscribe` /
  ` is not a valid product`). 30-minute OHLC and integer milli-BTC volume are
  derived only from executed `match` `price`, `size` and `time`. There is no
  REST bootstrap, REST history, REST OHLC or REST volume path. Startup has
  `bars=0 state=connecting`; on failure the last-good payload is kept with
  `fresh:false`.
- **Async JS host capabilities:** `na_fetchAsyncStart` / `na_onFetchResult` and
  `na_timerSchedule` / `na_timerCancel` / `na_onTimerTick`; the owner-scoped C
  ABI drain is `angnative_runtime_bridge_take_host_tasks(handle)` with JSON task
  kinds `fetch`, `timerSchedule` and `timerCancel`. The JS refresh request to
  `/marketpulse/quote` is served from the host's socket-derived cache, not a
  REST market endpoint.
- Real RxJS `interval(30000)` drives bounded refresh with one request in
  flight. WebSocket match pushes remain a separate path. No real-time guarantee
  is made.
- Heavy dataset: 10,000 deterministic rows generated in Angular, 200-row window
  emitted as native nodes. This is not a `RecyclerView` recycling claim.
- Animations: `provideNativeAnimations()` with deterministic `SetStyle` opacity
  samples `0 → 0.5 → 1` and `start,done` events. Real-time easing is not
  verified.
- A `Demo bars` control applies a deterministic fixture explicitly labelled
  `not live`; it is not presented as market data.

## Verification on physical OnePlus `b6640c8f` (2026-11-26)

| Check | Observed result |
|---|---|
| Install + launch | Final APK installed (`Success`) and launched; `Angular boot complete. nodes=213 plusNode=null` |
| Socket startup honesty | `MarketPulse waiting quote pushed: bars=0 state=connecting feed=Coinbase Exchange WebSocket · matches · 30m bars from executed matches · volume from match sizes error=none` |
| First live match | `MarketPulse fresh quote pushed: bars=1 state=live ... error=none` — no seeded history |
| UI status | `Angular in Rust: nodes=213 · Coinbase Exchange WebSocket · matches · 30m bars from executed matches · volume from match sizes · 1 30m bars · fresh · live` |
| Pair status | `BTC/USD · 1 30m match-derived bars · live · live · ... · not guaranteed real-time`; price `BTC/USD 60700.26` |
| Candle visibility | `Custom:na-candlestick` at `[168,826][1064,1134]`; 2-pixel sample of the valid 1116×2484 screenshot: `regionSamples=68992 greenPixels=0 redPixels=1518` — painted one-bar candle |
| UI node stability | `uiautomator` visible subset `nodes_total=43`, `textview_count=25`; native shadow-tree count `213`. Price/status update in the same native text node; no node growth claimed |
| ABI instrumentation | `$env:ANDROID_SERIAL='b6640c8f'; .\gradlew.bat connectedDebugAndroidTest --no-daemon` from `showcases\ecosystem-core` → `Running tests on devices: CPH2551 - 16`, `BUILD SUCCESSFUL in 1m 13s` |
| Physical sustained repeat | 11 apps, `status=ok`, 33/33 valid samples; MarketPulse 1699.3 frames, 2.2% janky, p50/p95 8/22 ms, 40.34% procstat CPU (one core), 38.45% top CPU, ΔPSS -8956 KB |
| Warm start | n=5, `process_ms` mean 336.0 ms, `TotalTime` mean 57.0 ms; same PID confirmed |
| JS gate | `pnpm test`: 16 files / 251 tests passed |
| Rust gate | `cargo test --workspace`: 154 passed / 0 failed / 17 ignored; ignored runtime-bridge suite 17 passed / 0 failed |
| FFI check | `FFI declaration check OK: 36 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 29 header declarations; iOS unified staticlib wiring OK` |
| Whitespace check | `git diff --check`: no whitespace errors (LF→CRLF working-copy warnings only) |

## Historical stages (not the active feed)

- CoinGecko BTC/USD OHLC returned 48 delayed 30m bars on `emulator-5554`; the
  nine-test generic MarketPulse suite passed then. Those results are retained
  in `docs/evidence/ANDROID-MARKETPULSE.md` sections 1-10 and are **not**
  claimed for the final socket APK.
- The earlier Yahoo stock stage returned HTTP 429 on-device and was superseded.
- A socket build before the subscription-format fix used object-form channels
  and failed to subscribe; it is superseded by the string-array subscription
  above.

## Honest limitations

- No real-time guarantee. WebSocket availability and Coinbase delivery are
  external dependencies; failure keeps last-good bars with `fresh:false`.
- No fabricated history or volume is used for market data. The `Demo bars`
  fixture is explicitly labelled `not live`.
- SSE is not implemented. No REST market-data path remains.
- Physical sustained figures come from one device with
  `physical/unknown renderer (ro.hardware=qcom)`. Procstat CPU is percent of
  one core; `top` is instantaneous; `gfxinfo` is cumulative after a
  post-settle reset; memory deltas include GC/settle effects; `process_ms` is
  not first-frame time. Low-frame samples were retained, not discarded.
- The final socket APK did **not** rerun the historical nine-test generic
  MarketPulse instrumentation suite. The ABI pass shown above is the
  `ecosystem-core` instrumentation suite.
- This is a native reimplementation/adapter of charting concepts, not unchanged
  ECharts/AG Grid/GSAP compatibility.
- `10,000` is the generated dataset size; the rendered window is bounded.
- iOS/Mac remains unverified locally; no macOS/Xcode is available.

## Artifacts

- `app-debug.apk` — final socket-only debug APK used for install/launch evidence.
- `source.zip` — current MarketPulse source, host/ABI changes, tests, plans and
  retained evidence for this run.
- `compatibility-report.md` — this file.
- Live UI evidence: `docs/assets/market-pulse-socket-oneplus.log`,
  `docs/assets/market-pulse-socket-oneplus-window.xml`,
  `docs/assets/market-pulse-socket-oneplus.png`.
- ABI/sustained evidence: `docs/assets/market-pulse-abi-oneplus.log`,
  `docs/assets/market-pulse-sustained-run.log`,
  `docs/benchmarks/sustained/b6640c8f-20261126-170816.json`.
- Full command/output record: `docs/evidence/ANDROID-MARKETPULSE.md` section 11
  and `docs/evidence/ANDROID-SUSTAINED-LOAD.md` physical section.
