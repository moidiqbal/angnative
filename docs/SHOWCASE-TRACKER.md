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
| Existing tests | 108 Rust tests; 208 JS tests; Android emulator UI + click verified |

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

## Phase 2 — Ecosystem showcases
⬜ 11 categories pending (see mission brief). Build-time tools get validation suites, not APKs.

## Phase 3 — Reusable packages
⬜ `@angnative/ui-core`, adapter packages, native-grid/charts/animation/richtext — incremental only.

## Phases 4-7
⬜ Compatibility matrix, APK build/verify, benchmarks, repository deliverables — pending.

## Evidence rules
- No APK reported as verified unless installed + launched on emulator.
- Every compatibility row records exact library version, API level, backend, tests, limits.
- Native reimplementations are labelled as such.
