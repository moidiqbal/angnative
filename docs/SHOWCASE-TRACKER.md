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
| Supported native widgets | `View`, `Text`, `Button`, `TextInput`, `Image`, `ScrollView`, `Switch`, `ActivityIndicator`, `List`, `Slot` (see `packages/contract/src/elements.ts` and `JniBridge.createView`) |
| Build system | Gradle 9.9.2 / AGP 9.5.2 / JDK 25 (latest profile) **or** Gradle 8.2 / AGP 8.2 / JDK 17 (legacy) |
| Android SDK | `C:\Android\Sdk`, API 34/36, NDK 27.2.12479018 |
| Web adapter | `@angnative/web` — DOM path only, explicitly NOT used for native APKs |
| Existing tests | 133 Rust tests (+7 ignored acceptance, all passing); 220 JS tests; Android emulator UI + click verified |

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

Microfrontends remain documented in the compatibility matrix as `Not yet tested` (requires
JS runtime); no APK claimed.

## Phase 3 — reusable infrastructure
| Package | Status |
|---|---|
| `@angnative/native-ui-core` (`packages/native-ui-core/`) | ✅ real Android library: 8 PlatformOp callbacks, pluggable view creator, theme, StyleCodec, 6 JUnit tests; AAR 40,554 bytes; `assembleRelease` + `test` BUILD SUCCESSFUL |
| Adapter packages (`material-adapter`, etc.) | ⬜ not created — rule "no empty placeholders": each needs a real native implementation first |

## Phase 5/6 — build/verify/performance
- All 8 showcase APKs staged under `dist/showcases/<name>/` with source.zip + compatibility-report.md; installed/launched on emulator API 34; `WebViews=0`.
- Initial cold-start/memory numbers in `docs/BENCHMARKS.md` + `docs/benchmarks/*.json` (1 sample, emulator, honest caveats).
- Frame pacing measured on emulator: `docs/benchmarks/frame-pacing.json` (gfxinfo jank/p50/p95 for grid, charts, richtext, calendar). Physical-device multi-sample and CPU still pending.

## Phase 3 — Reusable packages
⬜ `@angnative/ui-core`, adapter packages, native-grid/charts/animation/richtext — incremental only.

## Phases 4-7
⬜ Compatibility matrix, APK build/verify, benchmarks, repository deliverables — pending.

## Evidence rules
- No APK reported as verified unless installed + launched on emulator.
- Every compatibility row records exact library version, API level, backend, tests, limits.
- Native reimplementations are labelled as such.
