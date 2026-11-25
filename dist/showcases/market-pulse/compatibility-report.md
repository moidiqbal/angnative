# MarketPulse — compatibility / verification report (2026-11-25)

## Identity

| Field | Value |
|---|---|
| Showcase | MarketPulse — combined heavy windowed OHLC + Angular animations + native crypto candlestick |
| APK | `dist/showcases/market-pulse/app-debug.apk` (copied from `platforms/android/app/build/outputs/apk/debug/app-debug.apk`, 33,064,229 bytes) |
| APK SHA-256 | `3A648816480AFD6C7F040A95E27271D48E1E9956B0EB0093C8756A3B588D51ED` |
| Package ID | `com.angnative.app` (generic `platforms/android` shell, not a separate showcase package) |
| Build profile | latest: Gradle 9.9.2 / AGP 9.5.2 / JDK 25 |
| Evidence | `docs/evidence/ANDROID-MARKETPULSE.md` sections 7-8; designs `docs/plans/2026-11-25-marketpulse-design.md` and `docs/plans/2026-11-25-marketpulse-signals-rxjs-design.md` |

## What was actually built

- Real Angular AOT bundle (`dist/aot/market-pulse-bundle.js`, SHA-256
  `DA29DBA6DA910CDE645D72023E8CAB81D76170E27C7E300F9984DA311C45AF80`) executed
  by the Rust-owned QuickJS runtime. No WebView. The template uses explicit
  `[style.*]`/`[text]` bindings; without them the Rust layout emitted zero-size
  frames and the emulator showed a white screen.
- Two assets are staged: `assets/angnative-bundle.js` keeps the counter app and
  its instrumentation assertions (SHA-256
  `AD8C07D59ACE87A3A6E6E59FDBD060170FB5F8A2576A9B552B64A8B19E37A80F`), while
  `assets/market-pulse-bundle.js` is selected by the Android shell at runtime.
  The two bundle hashes match their `dist/aot` counterparts.
- Chart is the contract element `na-candlestick` (`Custom:na-candlestick`),
  created by `NativeViewFactory` and drawn with Android `Canvas`. OHLC bars cross
  the bridge as a JSON string on `attr:bars`.
- Active feed: CoinGecko public BTC/USD OHLC
  (`https://api.coingecko.com/api/v3/coins/bitcoin/ohlc?vs_currency=usd&days=1`).
  Rows `[timestamp_ms, open, high, low, close]` are normalized to the bars
  contract; volume is explicitly `0` because the endpoint does not return it.
  Kotlin prefetches on a worker thread, registers a synchronous JS fetch handler
  before bundle boot, and both manual refresh and the 30 s delayed Kotlin poll
  push through `__marketPulseApplyQuote(json)` into an RxJS `Subject`. Angular
  Signals derive symbol, price, status and the `attr:bars` JSON string. No new
  JNI async method or WebSocket/SSE was added.
- A `Demo bars` button applies a clearly-labelled deterministic BTC/USD fixture
  (`not live`) through the real Angular `(press)` listener. The price and status
  update in the same native text nodes; no new textbox is created.
- Heavy dataset: 10,000 deterministic rows generated in Angular, 200-row window
  emitted as native nodes. This is not a `RecyclerView` recycling claim.
- Animations: `provideNativeAnimations()` with deterministic `SetStyle` opacity
  samples `0 → 0.5 → 1` and `start,done` events. Real-time easing is not verified.

## Verification on emulator `emulator-5554` (API 34 x86_64)

| Check | Observed result |
|---|---|
| Install + launch | `Success`; `Starting: Intent { cmp=com.angnative.app/.MainActivity }` |
| Boot logcat | `Angular boot complete. nodes=213 plusNode=null` |
| Feed status | `BTC/USD · 48 bars · CoinGecko public OHLC · 30m bars`; price `BTC/USD 58024` |
| Candle visibility | `content-desc="Custom:na-candlestick"` at `[137,556][1029,805]`; 2-pixel sample inside those bounds: `green=927 red=1396 grid=2851` |
| Layout overlap | 36 `TextView`s; pairwise bounds comparison found 0 intersections |
| `Demo bars` tap | `dispatchNativeEvent(11, press) -> listeners invoked: 2`; status changed to `BTC/USD · 32 bars · deterministic BTC/USD fixture (not live)` |
| Same-textbox update | Price `BTC/USD 58024 → BTC/USD 60895`, same bounds `[137,456][1003,534]`, same `TextView` count 36; ignored QuickJS test asserts stable node id and unchanged node count |
| On-device factory test | `OK (1 test)` — `taggedCandlestickCreatesNativeViewAndParsesBarsString` |
| Full Android instrumentation | `OK (9 tests)` |
| Signals + RxJS stream | Ignored QuickJS acceptance asserts `__marketPulseStreamEvents()` `1 → 2`, signal-backed status, stable price node id and unchanged node count |
| Delayed polling | 3 logcat pushes at 30 s intervals (`bars=48` each); after polls price `BTC/USD 58182`, 36 `TextView`s unchanged, candles still painted (`green=793 red=1601 grid=2147`) |
| In-process QuickJS acceptance | `market_pulse_bundle_drives_candlestick_and_animation ... ok` (attr:bars string, 200-row window, RxJS stream events, stable price node id, deterministic animation samples) |
| JS gate | `pnpm test`: 16 files / 251 tests passed |
| Rust gate | `cargo test --workspace`: 151 passed / 0 failed / 16 ignored |
| FFI check | `FFI declaration check OK: 34 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 28 header declarations; iOS unified staticlib wiring OK` |
| Whitespace check | `git diff --check`: no whitespace errors (LF→CRLF working-copy warnings only) |

## Honest limitations

- CoinGecko is a public, unofficial, delayed endpoint. The 48 bars observed on
  the emulator are real returned data, not a real-time guarantee. Volume is not
  available from this endpoint and is explicitly `0`. Polling is Kotlin-scheduled
  because the QuickJS `setInterval` shim is one-shot; no RxJS `interval`,
  WebSocket or SSE claim is made.
- The `Demo bars` fixture is deterministic and labelled `not live`; it is not
  market data.
- The earlier Yahoo stock feed returned HTTP 429 on-device and is no longer the
  active feed. No stock live-bar claim is made.
- No physical-device MarketPulse run (OnePlus `b6640c8f` was offline), no iOS/Mac
  build or run, and no NativeScript/Flutter head-to-head comparison was performed.
- This is a native reimplementation/adapter of charting concepts, not unchanged
  AG Grid/ECharts/GSAP compatibility.
- `10,000` is the generated dataset size; the rendered window is bounded. The
  final observed native node count was `213`.

## Artifacts

- `app-debug.apk` — rebuilt debug APK used for install/launch evidence.
- `source.zip` — MarketPulse source and contract/host/test changes used by this run.
- `compatibility-report.md` — this file.
- Live UI evidence (outside the zip): `docs/assets/market-pulse-crypto-boot.png`,
  `docs/assets/market-pulse-crypto-boot.xml`,
  `docs/assets/market-pulse-crypto-demo.png`,
  `docs/assets/market-pulse-crypto-demo.xml`,
  `docs/assets/market-pulse-signals-poll.png`,
  `docs/assets/market-pulse-signals-poll.xml`; full command/output record in
  `docs/evidence/ANDROID-MARKETPULSE.md` sections 7-8.
