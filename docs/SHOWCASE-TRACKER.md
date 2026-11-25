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
| Existing tests | `cargo test --workspace`: 151 passed / 0 failed / 16 ignored; `pnpm test`: 251 passed; Android emulator UI + click verified |

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
| MarketPulse (combined heavy + animated + crypto candlestick demo) | `com.angnative.app` (generic shell) | ✅ APK built, installed, launched on `emulator-5554` (API 34 x86_64); CoinGecko BTC/USD OHLC returned 48 live 30m bars; final boot `nodes=213`, UI dump `content-desc="Custom:na-candlestick"` with green/red candle pixels; real `Demo bars` tap → 2 live Angular `press` listeners and same-price-textbox update `58024 → 60895`; Android suite 9/9; two-asset split keeps counter tests green. No real-time guarantee. Evidence: `docs/evidence/ANDROID-MARKETPULSE.md`; staged `dist/showcases/market-pulse/{app-debug.apk,source.zip,compatibility-report.md}`; screenshots `docs/assets/market-pulse-crypto-boot.png`, `docs/assets/market-pulse-crypto-demo.png` |

Microfrontends remain documented in the compatibility matrix as `Not yet tested` (requires
JS runtime); no APK claimed.

## Phase 3 — reusable infrastructure
| Package | Status |
|---|---|
| `@angnative/native-ui-core` (`packages/native-ui-core/`) | ✅ real Android library: 8 PlatformOp callbacks, pluggable view creator, theme, StyleCodec, 6 JUnit tests; AAR 40,554 bytes; `assembleRelease` + `test` BUILD SUCCESSFUL |
| Adapter packages (`material-adapter`, etc.) | ⬜ not created — rule "no empty placeholders": each needs a real native implementation first |

## Phase 5/6 — build/verify/performance
- All 8 showcase APKs staged under `dist/showcases/<name>/` with source.zip + compatibility-report.md; installed/launched on emulator API 34; `WebViews=0`.
- MarketPulse staged under `dist/showcases/market-pulse/` with APK + source.zip + compatibility-report.md; installed/launched on emulator API 34; `WebViews=0`. It uses the generic shell package `com.angnative.app`, not a separate showcase package.
- Initial cold-start/memory numbers in `docs/BENCHMARKS.md` + `docs/benchmarks/*.json` (1 sample, emulator, honest caveats).
- Frame pacing measured on emulator: `docs/benchmarks/frame-pacing.json` (single burst) and `docs/benchmarks/emulator-multisample.json` (3 bursts per app).
- Sustained load measured on emulator: `docs/benchmarks/sustained/emulator-20261125-143058.json` + `docs/evidence/ANDROID-SUSTAINED-LOAD.md` (10 apps, 5 cold + 5 warm starts, 3 × 30 s, `status=ok`, 30/30 valid). Physical-device multi-sample, sustained CPU and warm-start still pending.

## Phase 3 — Reusable packages
✅ `@angnative/native-ui-core` is a real tested Android library (line 63); adapter packages and remaining native widgets stay incremental-only — no empty placeholders.

## Phases 4-7
✅ Compatibility matrix, APK build/verify, initial benchmarks and repository deliverables are in place; only physical multi-sample/CPU benchmarking remains pending.

## Evidence rules
- No APK reported as verified unless installed + launched on emulator.
- Every compatibility row records exact library version, API level, backend, tests, limits.
- Native reimplementations are labelled as such.
