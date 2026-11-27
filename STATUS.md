# AngNative — Current Status (single source of truth)

**Last verified:** 2026-11-27
**Repo / worktree:** `C:\Users\Moid Iqbal\Projects\angnative` (feature worktree `C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing`)
**Branch:** `master` (priority #4 on local branch `feat/real-time-easing-scheduler`, based on `aa8f2bc`, and priority #5 on local branch `feat/quickjs-bytecode-loader`, based on `93932bf`; neither merged)

> This file exists because an older status list claimed that `@angnative/web`,
> the iOS shell and the Windows desktop app "were not created". That list is
> outdated. Below is the verified state with commands anyone can re-run.

---

## 1. `@angnative/web` — EXISTS, TESTED, AOT BROWSER-VERIFIED ✅

**Files** (`packages/web/`): `package.json`, `README.md`, `tsconfig.json`,
`vitest.config.ts`, and `src/{index,bootstrap,renderer,dom-backend,primitives}.ts`
plus `dom-backend.spec.ts` and `web.spec.ts`. The browser demo is now compiled
by real `ngc` via `tsconfig.aot.web.json` and bundled by
`scripts/bundle-web-aot.mjs`; `apps/demo-app/src/main.web.ts` no longer imports
`@angular/compiler`. Angular partial declarations in `node_modules` are linked
by `scripts/angular-linker-plugin.mjs` before bundling/testing.

```powershell
Get-ChildItem packages\web\src -File          # shows all files above
pnpm compile:aot:web                          # Ivy definitions verified
pnpm bundle:aot:web                           # JIT packages absent from metafile
pnpm --filter @angnative/web test             # 2 files / 6 tests passed
pnpm test                                     # 18 files / 266 tests passed
```

Browser verification (Playwright/Chromium) — no console or page errors:
```
Count: 0 -> Count: 1, even -> odd after clicking '+'
Requests: http://localhost:4173/ and http://localhost:4173/app.js only
script[type=module][fetchpriority=high], html lang=en
```
Screenshot: `docs/assets/web-aot-demo.png` (SHA-256
`F631441729078E8BB0B3E1106E7F13177BFBE551196FB729D45A1AD999E8C0BD`).
Bundle: `dist/web-demo/app.js` 353,854 bytes, SHA-256
`832465B703BDE83C563A5D1DFCE5FA29523AF006D8627C373851D9C42C2B245B`.
Automation: `scripts/verify-web-aot.py` + `scripts/serve-web.mjs` (`pnpm start`).
Evidence: `docs/evidence/WEB-AOT-MIGRATION.md`. The web size-budget gate is now
implemented and verified; see the verification summary and `docs/evidence/WEB-SIZE-BUDGET.md`.

---

## 2. iOS shell — CODE COMPLETE, MAC TEST PENDING ⚠️

**Files** (`platforms/ios/`): `Package.swift`, `Sources/AngNativeHost/SwiftFFI.swift`,
`Sources/AngNativeApp/{UIViewFactory,DemoViewController}.swift`,
`Sources/AngNativeDemo/main.swift`, `Tests/AngNativeTests/AngNativeTests.swift`.

Implementation: Swift 6 SwiftPM package, UInt64 C-ABI signatures, `free_string`
on every Rust string, 8 UIKit view-factory callbacks, layout + click → Rust batch,
unit tests. `SwiftFFI.swift` now also binds the five unified runtime-bridge C ABI
functions and exposes `NativeRuntimeBridge` (main-thread owner, borrowed host,
bundle load, event dispatch). `Package.swift` and `build-ios-native.sh` link the
unified `libangnative_runtime_bridge.a`, which bundles host-ffi, host-engine and
QuickJS; the old host-only archive is not linked.
`python scripts/check-ffi-declarations.py` statically verifies 36 JNI methods
across 2 Android shells, 22 Swift bindings (13 runtime bridge), 29 header
declarations and the unified staticlib wiring.

**Honest status:** no Mac here → Swift compile, `swift test`, Simulator/device
run and the runtime-bridge smoke test are **NOT performed**. First action on the
Mac: `python scripts/check-ffi-declarations.py`, then
`./scripts/build-ios-native.sh simulator` and
`cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost`.
See `docs/IOS-BUILD.md` for the full runtime-bridge smoke checklist.

---

## 3. Windows desktop app — COMPILES, TESTED, WINDOW OPENS ✅

Cargo IS installed here: `C:\Users\Moid Iqbal\.cargo\bin\cargo.exe` (nightly 2026-11-14).

```powershell
cargo test --manifest-path crates/angnative-desktop/Cargo.toml   # 11 passed
cargo run  --manifest-path crates/angnative-desktop/Cargo.toml   # window opens
```
Live run opens a window titled **"AngNative Desktop"**. A WGSL validation panic
(`positions[vertex_index]` const-array dynamic index) was found and fixed in
`crates/angnative-desktop/src/renderer.rs`.

---

## 4. Android shells — flagship device-verified; generic shell AOT-capable ✅

- Two verified toolchain profiles: `scripts/build-android-stack.ps1 -Stack latest|legacy`
  - latest: Gradle 9.9.2 / AGP 9.5.2 / **JDK 25** → BUILD SUCCESSFUL
  - legacy: Gradle 8.2 / AGP 8.2 / JDK 17 → BUILD SUCCESSFUL
- Emulator AVD `AngNative34` (API 34 x86_64), script `scripts/run-android-emulator.ps1`
- Flagship APK runs; UI is built from Rust `PlatformOp`s; the original imperative
  click round trip was verified (`onNativePress → SetText → "Hello World!"`)
- Screenshots: `docs/assets/android-demo.png`, `docs/assets/android-demo-clicked.png`
- Wrong-thread JNI guard: `connectedDebugAndroidTest` passed 2/2 on `emulator-5554` (Android 14, x86_64) and 2/2 on OnePlus Open `b6640c8f` (`CPH2551`, Android 16, arm64-v8a); evidence in `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md` and `docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`
- UI-thread marshalling: `ViewFactoryMainThreadTest.kt` records `Looper.myLooper()` inside `NativeViewSink` callbacks; full suite 4/4 pass on both devices. Evidence: `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`.
- Real tap → live Angular listener: on both devices logcat showed `Angular tap node=17 -> 1 listener(s)` and UI changed `Count: 0 → Count: 1`. Screenshot retained for emulator (`docs/assets/angular-real-tap-emulator-count1.png`); OnePlus logcat + before/after XML retained (`docs/assets/angular-real-tap-oneplus-count1.png` was 0 bytes after device disconnect, so no image claim).
- Generic `platforms/android` shell: uses the same runtime bridge as the flagship
  — creates `RuntimeBridge`, loads a staged Angular AOT asset, materializes
  Angular-emitted native ops, and walks the native ancestor chain on tap. Two
  assets are staged: `assets/angnative-bundle.js` (1,098,643 bytes, stable
  counter name asserted by instrumentation) and `assets/market-pulse-bundle.js`
  (1,180,071 bytes, selected at runtime for MarketPulse). Dist/staged hashes
  match. The active MarketPulse build (2026-11-26) is **socket-only**: Coinbase
  Exchange `wss://ws-feed.exchange.coinbase.com`, `matches` channel only; 30m
  OHLC and integer milli-BTC volume come from executed `match` price/size/time.
  No REST bootstrap, history, OHLC or volume path exists; startup reports
  `bars=0 state=connecting`, then the first real match produced `bars=1
  state=live` on physical OnePlus `b6640c8f` (Android 16 / arm64-v8a). Failure
  keeps last-good data with `fresh:false`. Real RxJS `interval(30000)` drives
  bounded refresh through host-backed async fetch/timer capabilities; no
  real-time guarantee is made. Physical log/UI/screenshot evidence is in
  `docs/assets/market-pulse-socket-oneplus.{log,xml,png}`. The final socket APK
  now has its own `OK (9 tests)` rerun on the API 34 x86_64 emulator
  (`docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md`); the physical OnePlus rerun
  remains outstanding. Evidence: `docs/evidence/ANDROID-MARKETPULSE.md` sections
  1-10 (historical) and 11 (active). `python scripts/check-ffi-declarations.py`
  checks both Android JNI bridges.
- Sustained-load benchmark: the historical emulator run
  (`docs/benchmarks/sustained/emulator-20261125-143058.json`, 10 apps,
  5 cold + 5 warm starts, 3 × 30 s, `status=ok`, 30/30 valid) is retained. The
  physical repeat on `b6640c8f` (2026-11-26, final socket-only APK) is
  `docs/benchmarks/sustained/b6640c8f-20261126-170816.json` — `status=ok`,
  33/33 valid samples, 0 errors across 11 apps. MarketPulse averaged 1699.3
  frames, 2.2% janky, p50/p95 8/22 ms, 40.34% procstat CPU (one core), 38.45%
  instantaneous top CPU, ΔPSS -8956 KB; warm `process_ms` mean 336.0 ms.
  Post-change ABI instrumentation on the same device passed
  (`Running tests on devices: CPH2551 - 16`, `BUILD SUCCESSFUL in 1m 13s`).
  Evidence: `docs/evidence/ANDROID-SUSTAINED-LOAD.md`,
  `docs/assets/market-pulse-sustained-run.log`,
  `docs/assets/market-pulse-abi-oneplus.log`.

---

## Windows `cargo test --workspace` note

`rquickjs-sys` calls `patch -p1 -f` in its build script. The `patch.exe` that
appears first on PATH on this machine (a WinGet link) has a manifest requiring
elevation (error 740). Git's copy does not. Workaround:

```powershell
$env:PATH = "C:\Program Files\Git\usr\bin;$env:PATH"
cargo build --workspace
cargo test --workspace   # 166 pass, 0 failed, 19 ignored; includes host-engine and desktop (11)
```

## Verification summary

| Item | Status |
|---|---|
| Rust core (all workspace crates incl. host-engine + runtime-bridge + desktop) | ✅ **166 passed / 0 failed / 19 ignored** (with Git `patch.exe` on PATH — see note above); all 19 ignored acceptance tests also passed with generated `dist/aot` bundles, including the AOT bytecode load-and-mutate test (evidence `docs/evidence/QUICKJS-BYTECODE-LOADER.md`) |
| JS/TS (18 spec files) | ✅ 266 tests pass |
| Android JNI + emulator/physical run | ✅ verified: wrong-thread JNI instrumentation passed 2/2 and full Android suite passed 4/4 on Android 14 emulator and OnePlus Open (Android 16, arm64-v8a); real tap reached a live Angular listener on both |
| Generic `platforms/android` AOT shell | ✅ build + device verified: two staged AOT assets (`market-pulse-bundle.js` 1,180,071 bytes), runtime-bridge JNI, UI from native ops, physical socket boot `nodes=213`, no WebView node in UI dumps, `Custom:na-candlestick` bounds `[168,826][1064,1134]` with painted red candle pixels, post-change ABI instrumentation passed on OnePlus, FFI checker covers both shells (36 JNI / 22 Swift / 29 headers). The final socket APK passed the generic nine-test suite on the API 34 emulator (`OK (9 tests)`, 4.77 s); physical rerun not done. Evidence: `docs/evidence/ANDROID-MARKETPULSE.md` section 11 and `docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md`. |
| MarketPulse combined crypto demo | ✅ **socket-only active build** installed/launched on physical OnePlus `b6640c8f`; APK SHA-256 `EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7`, length 33,149,157 bytes. Coinbase Exchange WebSocket `matches` only: no REST bootstrap/history/OHLC/volume; startup `bars=0 state=connecting`, first live match `bars=1 state=live`, price `BTC/USD 60700.26`, `fresh · live`, `Custom:na-candlestick` painted, deterministic animation samples `0 → 0.5 → 1`. Real RxJS `interval(30000)` + host-backed async fetch/timer capabilities; one refresh in flight. Historical CoinGecko/Signals evidence is retained separately. ⚠️ no real-time guarantee, no fabricated history/volume, no physical-device final-socket nine-test rerun, no iOS/Mac claim; emulator rerun `OK (9 tests)` passed. Artifacts: `dist/showcases/market-pulse/`, `docs/assets/market-pulse-socket-oneplus.{log,xml,png}`. Evidence: `docs/evidence/ANDROID-MARKETPULSE.md` section 11 and `docs/evidence/FINAL-SOCKET-9TEST-EMULATOR.md`. |
| Sustained-load benchmark | ✅ emulator `status=ok`, 30/30 valid samples across 10 apps (historical); ✅ physical OnePlus `b6640c8f` `status=ok`, 33/33 valid samples across 11 apps on the final socket APK. MarketPulse: 1699.3 frames, 2.2% janky, p50/p95 8/22 ms, 40.34% procstat CPU (one core), warm `process_ms` mean 336.0 ms. Evidence: `docs/evidence/ANDROID-SUSTAINED-LOAD.md` and `docs/benchmarks/sustained/b6640c8f-20261126-170816.json`. |
| Desktop window | ✅ verified |
| Web adapter (browser) | ✅ verified — AOT/JIT-free bundle (`app.js` 353,854 bytes, SHA-256 `832465B703BDE83C563A5D1DFCE5FA29523AF006D8627C373851D9C42C2B245B`), Playwright click and request check; evidence `docs/evidence/WEB-AOT-MIGRATION.md` |
| Web size-budget gate | ✅ `pnpm check:web-size`: 112,461 B gzip / 128,000 B budget, 15,539 B headroom, exit 0 on the priority #4 worktree with the core easing change; 9 negative/positive tests; CI `size` job wired; evidence `docs/evidence/WEB-SIZE-BUDGET.md` and `docs/evidence/REAL-TIME-EASING.md` §5 |
| iOS shell | ⚠️ Swift runtime-bridge bindings + unified staticlib wiring statically verified; Swift compile / Simulator / device run NOT verified (no Mac/Xcode here). Runbook in `docs/IOS-BUILD.md`. |
| Native event bridge + runtime bridge | ✅ `angnative-runtime-bridge` owns Host + QuickJS; one delivery per event (canonical JS thread-local path + legacy FFI fallback); 19 ignored tests are present; the Angular acceptance set (bootstrap + click round trip + forms/ngModel/router/animations load + animations engine/timed easing/http/router app/i18n + AOT bytecode + MarketPulse) passes once `dist/aot` bundles exist, plus 1 third-party RxJS spot-check via `pnpm bundle:third-party`; shipping shell tap verified on emulator and OnePlus (`docs/EVENT-BRIDGE.md`) |
| Real-time easing scheduler (priority #4) | ✅ Implemented on local `feat/real-time-easing-scheduler` branch (not merged to `master`): recognized non-linear easing names use `na_nowMs()` + rAF host frames; linear/empty easing keeps deterministic `0 → 0.5 → 1`; no JNI/FFI/protocol/platform additions. Verified by 5 fake-clock unit tests, a timed renderer test, ignored AOT acceptance test, `pnpm test` 18 files / 266 tests, current worktree `cargo test --workspace` 166 pass / 0 failed / 19 ignored, FFI check 36 JNI / 22 Swift / 29 headers, web gate 112,461 B gzip / 128,000 B. Evidence: `docs/evidence/REAL-TIME-EASING.md`. |
| QuickJS bytecode loader (priority #5) | ✅ Implemented on local `feat/quickjs-bytecode-loader` branch (from `93932bf`, not merged): versioned `ANGNQJSB1` container with length/empty/oversize validation, `JsEngine::load_bytecode()` with an `Unsupported` default, QuickJS module-bytecode load + evaluate + job/microtask drain, matching `AngNativeRuntime`/`RuntimeBridge` methods, and a Rust `dump-bytecode` generator wired through `pnpm bundle:bytecode` and CI. AOT artifact `dist/aot/animations-app-bundle.qjsbc` is 1,412,561 B with SHA-256 `174F0B970E1BC0F295208FC3FD0AB169AE09332496704C9AE07C05A812AD9172`; the ignored acceptance test boots it and verifies live native `opacity` mutation plus `start,done`. Gates: 37 host-engine tests, 12 bridge tests / 19 ignored, 19/19 ignored suite, workspace 166 pass / 0 failed / 19 ignored, `pnpm test` 18 files / 266 tests, web gate 112,461 B gzip / 128,000 B, FFI counts unchanged. No C ABI/JNI/Swift/header, startup-speed, signature, ES-module-loader or platform-shell claim. Evidence: `docs/evidence/QUICKJS-BYTECODE-LOADER.md`. |
| Angular HttpClient DI (`NG0201`) in QuickJS | ✅ verified by ignored acceptance test `angular_http_platform_loads_in_quickjs`: `provideHttpClient()` + `INJECTOR_SCOPE: 'root'` resolve `HttpClient`, and `get('/api/x')` delivers the native fetch body (`hello-from-native|GET`) plus `HttpResponse.status === 200`. Exact root cause and commands in `docs/architecture/ARCHITECTURE-VERIFIED-2026-11-22.md`. |
| Template-driven `ngModel` in QuickJS | ✅ verified by ignored acceptance test `angular_template_driven_forms_round_trip_in_quickjs`: AOT `FormsAppComponent` uses `FormsModule`/`ngModel` on `<input>`; JS model changes emit native `SetProperty`/text mutations, and a Rust-dispatched native `input` event updates the model. `<input>` now maps to the `na-text-input` primitive; `navigator.userAgent` shim added for Angular's DefaultValueAccessor. |
| AOT i18n markers in QuickJS | ✅ verified by ignored acceptance test `angular_aot_i18n_translated_text_in_native_tree`: `tsconfig.aot.i18n.json` compiles `<na-text i18n>Hello {{ name }}!</na-text>` to `$localize` + `ɵɵi18n`/`ɵɵi18nExp`; a registered translation renders `Assalam-o-Alaikum AngNative!` in the native tree. Root causes: `$localize` placeholder self-substitution loop and missing `Node` DOM constants. |
| Angular Animations engine in QuickJS | ✅ verified by ignored acceptance test `angular_animation_engine_interpolates_native_styles`: `pnpm compile:aot:animations && pnpm bundle:framework` builds the AOT `fade` trigger; the real `AnimationEngine` over `AngNativeRendererFactory2` emits a native `SetStyle opacity: 0.5` midpoint between lifecycle `start,done`, ending at native `opacity: 1`. Deterministic linear animations still use three samples (`0 / 0.5 / 1`) via flat numeric-id registries. Named non-linear easing now opts into a host-clock rAF scheduler verified by `angular_timed_easing_drives_native_styles`: ease-in-out frames end at native opacity `0.125 / 0.5 / 0.875 / 1` at controlled 25/50/75/100 ms, with `start,done` and final native opacity `1`. No 60 FPS/vsync/GPU/platform claim. Evidence: `docs/evidence/REAL-TIME-EASING.md`. |
| Third-party library spot-check (RxJS 7.8.2) | ✅ verified in embedded QuickJS (native JS path) | `pnpm bundle:third-party` + ignored test `third_party_rxjs_runs_unchanged_in_quickjs`: unmodified RxJS `of(1,2,3).pipe(map(x => x*2))` yields `[2,4,6]` with `complete`, no error. Single DOM-free library only — no DOM-heavy UI library or third-party app was run unchanged; see `docs/COMPATIBILITY.md` §2b. |
| Android cross-thread bridge safety | ✅ verified on Android emulator and physical OnePlus Open | Fixed the release `panic = "abort"` defect: C ABI entry points reject wrong-thread calls with a sentinel before any `assert_owner_thread()` panic can occur (`test_c_abi_dispatch_from_non_owner_thread_returns_sentinel_without_panicking` + `test_thread_affinity` pass). Added `RuntimeBridgeThreadAffinityTest.kt`; `connectedDebugAndroidTest` passed 2/2 on `emulator-5554` (Android 14, x86_64) after rebuilding stale `jniLibs`, and 2/2 on OnePlus Open `b6640c8f` (`CPH2551`, Android 16, arm64-v8a). Evidence: `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md` and `docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`. UI-thread marshalling and a native tap into a live Angular bridge are now verified on both devices; see `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`. |
| Production AOT (`pnpm run compile:aot`, `angnative build`) | ✅ real Angular `ngc` (not shadowed 0.1.0 binary), Ivy definitions verified in `dist/aot/`, esbuild bundle produced |

Full history: `docs/PROJECT-REVIEW-2026-11-20.md`
Roadmap: `docs/ROADMAP.md`
