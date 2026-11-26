# MarketPulse — Heavy + Animated + Crypto Demo Design (2026-11-25)

> **Superseding decision (2026-11-26):** The user approved a fully socket-only
> MarketPulse target design. Market data comes only from the Coinbase Exchange
> WebSocket `matches` channel for BTC-USD executed matches; each `match` message
> supplies price, size and time, and both the 30-minute OHLC bars and BTC volume
> are derived from those match messages only. There is no REST market-data fetch
> in the target design. The earlier CoinGecko/REST bootstrap, host-fetch adapter,
> last-good REST caching, delayed Kotlin polling and one-shot-timer rationale in
> this document are superseded and are no longer the target design. This note
> records the approved decision, which was subsequently implemented and
> verified on 2026-11-26 in `docs/evidence/ANDROID-MARKETPULSE.md` section 11.

Status: **historical design, superseded by the socket-only implementation.**
Sections 1-2 were approved in chat on 2026-11-25 and describe the earlier
CoinGecko stage: 48 delayed BTC/USD 30-minute bars and visible candles were
verified on `emulator-5554`; the Yahoo stock endpoint returned HTTP 429. The
active build is socket-only Coinbase `matches` with match-derived 30m OHLC and
volume; exact commands, outputs, hashes and limitations are in
`docs/evidence/ANDROID-MARKETPULSE.md` section 11.

## Section 1 — Scope & data flow

- **Working name:** MarketPulse. One demo combining a heavy windowed OHLC dataset, Angular `provideNativeAnimations()` samples, and a native candlestick chart.
- **Shell:** reuse `platforms/android` generic shell. `AngNativeView.kt` boots an Angular AOT bundle from app assets and selects between `assets/angnative-bundle.js` (counter, stable name asserted by instrumentation) and `assets/market-pulse-bundle.js` (this demo); the Kotlin layer only adds the native chart view and the host-fetch adapter. Do **not** copy the large hand-built `showcases/ecosystem-core` Activity.
- **Decision:** the chart is a **proper `na-candlestick` contract element**, not a one-off injected Kotlin view. This is a cross-layer change (TS contract → Rust node/op → Android view).
- **Data source:** public, no-key CoinGecko BTC/USD OHLC endpoint, verified from the host and on-device on 2026-11-25:
  `https://api.coingecko.com/api/v3/coins/bitcoin/ohlc?vs_currency=usd&days=1` → HTTP 200, rows `[timestamp_ms, open, high, low, close]` spaced 30 minutes. Coinbase Exchange also returned HTTP 200 but uses `[time, low, high, open, close, volume]`, so CoinGecko was chosen for the simpler honest mapping.
  This endpoint is **public, unofficial and delayed**. UI label is `CoinGecko public OHLC · 30m bars`; no real-time guarantee. CoinGecko has no volume column, so normalized `v` is `0` rather than invented.
- **Flow:** Kotlin prefetches/normalizes quotes off the main thread → registers a synchronous JS fetch handler before bundle boot → Angular `HttpClient` reads that host-cached response → normalized OHLC string is bound onto `na-candlestick` → Android Canvas draws candles. Refresh re-fetches in Kotlin and pushes fresh JSON into Angular through a bundle hook.
- **Heavy part:** Angular generates up to 10,000 deterministic OHLC rows, but renders a bounded window (default 200 rows) as `na-view`/`na-text` nodes. The claim will be **"N Angular-created native nodes observed"** with N taken from a real logcat/snapshot; it is not a `RecyclerView` recycling claim.
- **Animations:** use the existing `provideNativeAnimations()` pipeline. Claims are limited to deterministic `SetStyle` opacity samples `0 → 0.5 → 1` and `start,done` callbacks, exactly as in the ignored acceptance test in `crates/angnative-runtime-bridge/src/lib.rs:1443-1540`. No 60 FPS, smooth easing or wall-clock tween claim.
- **Output:** staged `dist/showcases/market-pulse/` (APK, source.zip, compatibility-report.md), tracker row, evidence doc. Emulator first; physical device remains parked.

## Section 2 — `na-candlestick` contract, props, fetch & failure handling

### 2.1 Contract touch-points (verified against current code)

| Layer | Exact change |
|---|---|
| `packages/contract/src/elements.ts:19-30,122-256` | Add `candlestick: 'na-candlestick'` to `ELEMENT_TAGS` and one contract: `kind: 'custom'`, `nodeType: 'Custom:na-candlestick'`, `defaultStyles: { width: '320px', height: '220px' }`, `allowedChildren: []`, `capabilities: ['chart']`, `recyclesView: false`, `hostsText: false`. `width`/`height` are valid shared style keys (`packages/contract/src/style.ts:127-128`). |
| `packages/core/src/renderer.ts:75-116` | Add `case 'na-candlestick': return 'Custom:na-candlestick'`. Without it, `tagToNodeType` returns untagged `Custom`, which `crates/angnative-host-view/src/op_emitter.rs:21-26` flattens — no native view would be created. |
| `crates/angnative-host-core/src/node.rs:49-63` | No change needed: `from_wire` already accepts any `Custom:<tag>`. Mirror `from_tag` in `crates/angnative-core/src/node.rs:19-29` for consistency (grep shows that copy is not called elsewhere). Update the `op_emitter.rs:21-23` comment to name the new tagged primitive. |
| `platforms/android/app/src/main/java/com/angnative/app/NativeViewFactory.kt:39-44,59-73,202` | Add `CUSTOM_CANDLESTICK_TAG = "Custom:na-candlestick"` and create a new `CandlestickView(context)` in `createViewForType`. It is a leaf `View` with `onDraw(Canvas)`: background grid, high→low wick, open→close body, green when `close >= open` else red. No DOM/WebView. |
| `packages/web/src/renderer.ts:130-148` | No special case required: known elements fall through to the core renderer; the web backend is not part of the Android evidence path. |

### 2.2 OHLC property format

Data crosses the bridge as a **JSON string**, not a JSON array: `NativeViewFactory.kt:87-109` iterates `ViewProps.properties` with `JSONObject.optString`, which drops a `JSONArray` value. Angular binds `[attr.bars]="quoteJson"`; `renderer.ts:456-470` emits `SetProperty { key: 'attr:bars', value: string }`; `op_emitter.rs:247-260` puts it into `ViewProps.properties`; Kotlin parses it.

Payload (normalized by Kotlin before it reaches Angular):

```json
{
  "symbol": "BTC/USD",
  "feed": "CoinGecko public OHLC · 30m bars",
  "fetchedAt": "2026-11-25T...Z",
  "bars": [
    { "t": 1795530600000, "o": 59953.0, "h": 60419.0, "l": 59940.0, "c": 60362.0, "v": 0 }
  ]
}
```

- Chart window: at most 60 bars are sent; the Angular component slices the dataset. Each update replaces the whole array — no diff protocol, no partial updates.
- Colors are fixed constants in the native view for v1, so the shared style table is untouched.
- `onNativeSetProps` handles only keys `bars` / `attr:bars` for `CandlestickView`; other keys keep the existing generic style behavior.
- Test surface: `NativeViewFactory.viewFor(nodeId)` already exists (`:180-181`); add a readable bar count/state on `CandlestickView` for instrumentation assertions.

### 2.3 Host fetch adapter & failure handling

Verified constraint: `crates/angnative-host-engine/src/quickjs_shims.rs:226-317` invokes fetch handlers **synchronously** and wraps their result in a Promise; there is no registered Android fetch handler today (repo-wide grep found only the shim and test registrations). `JniBridge.kt` exposes `loadScript` (`:204-208`) and event dispatch, but no generic JS→host fetch call.

Chosen adapter (no new JNI method):

1. Before `jniBridge.loadScript(bundle)`, `AngNativeView.kt` triggers a Kotlin prefetch on a background executor (`HttpURLConnection`, 8 s connect/read, HTTP 200 check, required-array shape validation). On success it evaluates a small bootstrap script that registers `__angnativeRegisterFetchHandler(function (req) { ... })` returning the cached normalized payload for the demo URL; unmatched URLs return status 502 with an error body.
2. The Angular AOT bundle uses real `HttpClient.get('/marketpulse/quote')` through that host handler. This is honest: the network work happened in Kotlin; the first JS response is host-cached.
3. A refresh `na-pressable` carries `attr:data-action="refresh"`. `NativeViewFactory` records that action by node id; the tap walk in `AngNativeView.kt` recognizes it, starts an async Kotlin refetch, then evaluates `__marketPulseApplyQuote(<json>)` via `jniBridge.loadScript` and re-runs layout. A second `na-pressable` with `data-action="demo-bars"` binds `(press)` to a clearly-labelled deterministic BTC fixture; the tap walk dispatches both `click` and `press` while walking ancestors because `NaPressableComponent` listens for `press`. The bundle hook updates Angular state so the new `bars` string is emitted as a real renderer mutation, and the existing price text node is updated in place.
4. Failure: never fabricate bars. On timeout, non-200, malformed JSON or missing arrays, keep the last good payload, show `feed unavailable: <reason>` in an `na-text` status node, and log the cause through `Log.w`. The fetch handler returns an explicit error status for that attempt.
5. Not in scope: changing the fetch shim to accept promises or adding a `na_fetchAsync` JNI callback. That would be required for a true async `HttpClient → host network` claim and is explicitly deferred.

### 2.4 Validation plan (execution gates)

| Gate | Required proof |
|---|---|
| TS | `pnpm test`: contract test for `isKnownElement('na-candlestick')` and `nodeType`; `renderer.spec.ts` `tagToNodeType('na-candlestick') === 'Custom:na-candlestick'`; conformance fixture with `CreateNode` + `SetProperty attr:bars` string. |
| Rust | `cargo test --workspace`: `op_emitter` test proving an emitted `Create` for `Custom:na-candlestick` plus `SetProps` carrying the bars string; `NodeType::from_wire` tagged-custom test still green. |
| QuickJS | Ignored/feature acceptance test loading the MarketPulse AOT bundle: assert a `SetProperty attr:bars` string mutation, and re-run the deterministic animation assertion (`0, 0.5, 1`; `start,done`) so the new element does not regress it. |
| Android build | `.\gradlew.bat assembleDebug assembleDebugAndroidTest --no-daemon` → BUILD SUCCESSFUL; APK stages both `assets/angnative-bundle.js` and `assets/market-pulse-bundle.js`; hashes verified against `dist/aot/*.js`. |
| Device | `adb install` + launch on `emulator-5554`; `dumpsys` WebView count = 0; logcat with actual node count; UI hierarchy contains `content-desc="Custom:na-candlestick"`; screenshot staged under `docs/assets/market-pulse-emulator.png`. On-device OHLC bar parsing is asserted by `CandlestickViewFactoryTest`, not by a live feed count. |
| Docs | Evidence doc with exact commands and outputs (`docs/evidence/ANDROID-MARKETPULSE.md`); `docs/SHOWCASE-TRACKER.md` row after install+launch; staged `dist/showcases/market-pulse/`; stale `tasklist.txt` tail cleaned; benchmark map entry only if a sustained run is actually performed. |

## Section 3 — Honesty gates

Claim only what was executed:

- Allowed after evidence: "APK builds/installs/launches", "N Angular-created native nodes", "tagged `na-candlestick` created and its bars string parsed on-device", "WebViews: 0", measured CPU/memory if a benchmark is run.
- Not allowed: live device bars when the endpoint is rate-limited; real-time/delayed guarantee beyond the labelled source; smooth 60 FPS easing; 10,000 native rows (the dataset is 10,000, the rendered window is measured); `RecyclerView` recycling; AG Grid/ECharts/GSAP compatibility; physical-device results; generic-shell device verification before the emulator run.

Implementation order: contract + TS tests → Rust op/test → `CandlestickView` + factory + instrumentation test → Angular AOT entry/bundle + host fetch adapter → build/stage APK → emulator evidence → docs updates.
