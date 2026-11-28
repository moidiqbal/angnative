# Native Library Compatibility & APK Showcase — Tracker

**Constraint:** Zero WebView for native-mode demos. Capacitor/WebView approach explicitly rejected.

## Phase 0 — Architecture inspection (evidence-backed)

| Area | Actual state (verified) |
|---|---|
| Renderer | `packages/core` — custom Angular `Renderer2`, emits `CommandBatch` JSON |
| Shadow tree | Rust `angnative-host-core` (`MutationEngine`) + `angnative-host-view` (`OpEmitter`) |
| Layout | `angnative-host-layout`, **Taffy 0.15.1** (docs corrected from 0.5.2) |
| Native bridge | `angnative-host-ffi` C ABI + JNI (`platforms/android`) + Swift (`platforms/ios`) |
| Android UI | `JniBridge.kt`, `NativeViewFactory.kt`, `AngNativeView.kt` — views created from Rust `PlatformOp`s, not hand-built |
| Supported native widgets | `View`, `Text`, `Button`, `TextInput`, `Image`, `ScrollView`, `Switch`, `ActivityIndicator`, `List`, `Slot`, `Candlestick` (`na-candlestick` → `Custom:na-candlestick`, see `packages/contract/src/elements.ts` and `NativeViewFactory.kt`) |
| Build system | Gradle 9.9.2 / AGP 9.5.2 / JDK 25 (latest profile) **or** Gradle 8.2 / AGP 8.2 / JDK 17 (legacy) |
| Android SDK | `C:\Android\Sdk`, API 34/36, NDK 27.2.12479018 |
| Web adapter | `@angnative/web` — DOM path only, explicitly NOT used for native APKs |
| Existing tests | `cargo test --workspace`: 174 passed / 0 failed / 19 ignored (ignored runtime-bridge suite 19 passed); `pnpm test`: 266 passed (18 files); Android emulator + physical OnePlus UI/tap/ABI verified |

### DOM-dependent libraries — verdict before implementation
Angular Material, PrimeNG, NG-ZORRO, Taiga UI, Spartan UI, AG Grid, ECharts,
Tiptap, FullCalendar, GSAP, Formly — all DOM/CSS based. **None can run unchanged**
on the native renderer. Each native showcase is therefore an explicitly labelled
**native reimplementation / adapter**, never "unchanged compatibility".

Runtime-capable unchanged on native: state libs (SignalStore/NGRX/NGXS/Elf),
RxJS, Transloco, i18n, testing tools (build-time), Angular CDK concepts (partial).

## Phase 1 — UI showcase APKs

| Showcase | Package ID | Status |
|---|---|---|
| Angular Material | `com.angnative.demo.material` | ✅ **POC built, installed, launched, verified** — APK 19.21 MB, `WebViews: 0`, screenshot `docs/assets/showcase-material.png`, report `dist/showcases/material/compatibility-report.md`, source `showcases/material/` + `source.zip` |
| PrimeNG | `com.angnative.demo.primeng` | ✅ built + installed + launched, WebViews=0, 19.21 MB |
| NG-ZORRO | `com.angnative.demo.ngzorro` | ✅ built + installed + launched, WebViews=0, 19.21 MB |
| Taiga UI | `com.angnative.demo.taiga` | ✅ built + installed + launched, WebViews=0, 19.21 MB |
| Spartan UI | `com.angnative.demo.spartan` | ✅ built + installed + launched, WebViews=0, 19.21 MB |

All five: `dist/showcases/<name>/{app-debug.apk,source.zip,compatibility-report.md}`,
screenshots `docs/assets/showcase-<name>.png`, source `showcases/<name>/`.
Build/verify scripts: `scripts/generate-showcase.ps1`, `scripts/build-showcases.ps1`,
`scripts/verify-showcases.ps1`, `scripts/generate-showcase-reports.ps1`.

## Phase 2 — Ecosystem showcases (2026-11-21)

| Showcase | Package ID | Status |
|---|---|---|
| Native Grid (AG Grid equivalent) | `com.angnative.demo.ecosystem.grid` | ✅ APK built, installed, scrolled (177 views recycled), WebViews=0, report + source.zip |
| Native Charts (ECharts equivalent) | `com.angnative.demo.ecosystem.charts` | ✅ APK built, installed, bar/line/donut via native Canvas, WebViews=0, report + source.zip |
| Ecosystem Core (state/i18n/forms/animation) | `com.angnative.demo.ecosystem.core` | ✅ APK built, installed, native widgets + JNI, WebViews=0, report + source.zip |
| Build-time tools (Nx/Angular CLI/Playwright/Vitest/Jest) | n/a | ✅ `scripts/validate-build-tools.mjs` → 25 PASS / 0 FAIL / 6 INFO; docs `BUILD-TOOLS-VALIDATION.md` |
| Ecosystem compatibility matrix (11 categories) | n/a | ✅ `docs/ECOSYSTEM-COMPATIBILITY.md` (honest labels, no fabricated results) |

| Rich text | `com.angnative.demo.ecosystem.richtext` | ✅ APK built, installed, launched; native `SpannableString` editor (bold/italic/underline/bullet/color/size/link/undo); WebViews=0; report + source.zip |
| Calendar | `com.angnative.demo.ecosystem.calendar` | ✅ APK built, installed, launched; native Canvas month grid with event dots/selection; WebViews=0; report + source.zip |
| MarketPulse (combined heavy + animated + crypto candlestick demo) | `com.angnative.app` (generic shell) | ✅ **active socket-only build** staged/installed/launched on physical OnePlus `b6640c8f` (Android 16 / arm64-v8a), APK SHA-256 `EDD184C13205029483389F2B20FA2BCB3EFDB6C5D4BEB3E107C32A17EEB387B7`, 33,149,157 bytes. Coinbase Exchange WebSocket `matches` only: no REST bootstrap/history/OHLC/volume; startup `bars=0 state=connecting`, first live match `bars=1 state=live`, `BTC/USD 60700.26`, `fresh · live`, `Custom:na-candlestick` bounds `[168,826][1064,1134]` with painted match-derived candle. Real RxJS `interval(30000)` + host-backed async fetch/timer capabilities; one refresh in flight. Post-change ABI instrumentation passed on device; physical sustained repeat `status=ok`, 33/33 valid, MarketPulse 1699.3 frames / 2.2% janky / p50-p95 8-22 ms / 40.34% procstat CPU (one core) / warm `process_ms` mean 336.0 ms. ✅ final-socket generic nine-test suite passed on both the API 34 emulator and physical OnePlus `b6640c8f` (`OK (9 tests)`; `docs/evidence/FINAL-SOCKET-9TEST-ONEPLUS.md`); ⚠️ no real-time guarantee, no fabricated history/volume, no iOS/Mac claim. Historical CoinGecko/Signals evidence retained in `docs/evidence/ANDROID-MARKETPULSE.md` sections 1-10; active evidence in section 11 and retained artifacts `docs/assets/market-pulse-socket-oneplus.{log,xml,png}`, `docs/assets/market-pulse-abi-oneplus.log`, `docs/assets/market-pulse-final-socket-9test-oneplus.log`, `docs/assets/market-pulse-sustained-run.log`, `docs/benchmarks/sustained/b6640c8f-20261126-170816.json`. |

Microfrontends remain documented in the compatibility matrix as `Not yet tested` (requires
JS runtime); no APK claimed.

## Phase 3 — reusable infrastructure
| Package | Status |
|---|---|
| `@angnative/native-ui-core` (`packages/native-ui-core/`) | ✅ real Android library: 8 PlatformOp callbacks, pluggable view creator, theme, StyleCodec, 6 JUnit tests; AAR 40,554 bytes; `assembleRelease` + `test` BUILD SUCCESSFUL |
| Adapter packages (`material-adapter`, etc.) | ⬜ not created — rule "no empty placeholders": each needs a real native implementation first |

## Phase 5/6 — build/verify/performance
- All 8 showcase APKs staged under `dist/showcases/<name>/` with source.zip + compatibility-report.md; installed/launched on emulator API 34; `WebViews=0`.
- MarketPulse staged under `dist/showcases/market-pulse/` with APK + source.zip + compatibility-report.md; active socket-only APK installed/launched on physical OnePlus `b6640c8f`; no WebView node in UI dumps. It uses the generic shell package `com.angnative.app`, not a separate showcase package.
- Initial cold-start/memory numbers in `docs/BENCHMARKS.md` + `docs/benchmarks/*.json` (1 sample, emulator, honest caveats).
- Frame pacing measured on emulator: `docs/benchmarks/frame-pacing.json` (single burst) and `docs/benchmarks/emulator-multisample.json` (3 bursts per app).
- Sustained load measured on emulator: `docs/benchmarks/sustained/emulator-20261125-143058.json` (10 apps, 5 cold + 5 warm starts, 3 × 30 s, `status=ok`, 30/30 valid, historical). Physical repeat on `b6640c8f`: `docs/benchmarks/sustained/b6640c8f-20261126-170816.json` (11 apps, `status=ok`, 33/33 valid, final socket-only MarketPulse APK). Evidence: `docs/evidence/ANDROID-SUSTAINED-LOAD.md`.

## Phase 3 — Reusable packages
✅ `@angnative/native-ui-core` is a real tested Android library (line 63); adapter packages and remaining native widgets stay incremental-only — no empty placeholders.

## Phases 4-7
✅ Compatibility matrix, APK build/verify, initial benchmarks and repository deliverables are in place; physical multi-sample/CPU/warm-start benchmarking is now recorded for OnePlus `b6640c8f` on the final socket-only MarketPulse APK. iOS/Mac remains unverified locally.

## Evidence rules
- No APK reported as verified unless installed + launched on emulator.
- Every compatibility row records exact library version, API level, backend, tests, limits.
- Native reimplementations are labelled as such.
