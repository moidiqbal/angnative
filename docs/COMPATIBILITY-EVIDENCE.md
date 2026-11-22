# Phase 4 — Compatibility Evidence Matrix

**Date:** 2026-11-21
**Scope:** every showcase APK actually built and verified in this repository.
**Rule applied:** no compatibility claim without execution evidence; native reimplementations are never labelled "unchanged library compatibility".

## Environment (exact)

| Item | Version / value |
|---|---|
| Angular | `@angular/core` 22.2.5, `@angular/compiler-cli` 22.2.5 |
| TypeScript | 6.0.3 |
| Rust | nightly-2026-11-14 (cargo 1.102.0-nightly) |
| Taffy | 0.15.1 (workspace `Cargo.toml`) |
| Gradle | 9.9.2 (latest profile) / 8.2 (legacy profile) |
| AGP | 9.5.2 (latest) / 8.2.0 (legacy) |
| JDK | 25.0.5.1 (latest) / Temurin 17.0.21 (legacy) |
| Android NDK | 27.2.12479018 |
| Android SDK | API 34 / 36, build-tools 34.0.0 / 37.0.0 |
| Emulator | `sdk_gphone64_x86_64`, API 34, 1080×2400 @ 420 dpi, AVD `AngNative34` |
| Test suites | `pnpm test` 220 passed; `cargo test --workspace` 134 passed (with Git `patch.exe` on PATH); 8 ignored Angular-in-QuickJS acceptance tests pass separately |

## Phase 1 — UI library showcases

| Feature / showcase | Library + version tested | Android API | Renderer / backend | Native implementation | Tests + execution evidence | Known limitations | Perf |
|---|---|---|---|---|---|---|---|
| Angular Material | Angular Material — **design reference only, not bundled** (DOM library) | min 24, target 34 | Rust host → JNI → `NativeViewFactory` → `TextView`/`Button`/`FrameLayout` | Native cards, buttons, Material-like palette/typography | APK built, installed, launched; `docs/assets/showcase-material.png`; `WebViews: 0`; report `dist/showcases/material/compatibility-report.md` | Native equivalent, not unchanged Material runtime; inputs/tabs/dialogs/virtual scroll not implemented | `docs/benchmarks/` (sized, single sample) |
| PrimeNG | PrimeNG — design reference only (DOM library) | min 24, target 34 | same JNI native path | same native primitives, PrimeNG branding | built/installed/launched, `showcase-primeng.png`, WebViews=0 | Tables/dialogs not implemented | not separately measured |
| NG-ZORRO | NG-ZORRO — design reference only (DOM library) | min 24, target 34 | same JNI native path | same native primitives, NG-ZORRO branding | built/installed/launched, `showcase-ngzorro.png`, WebViews=0 | Ant Design tokens not fully mapped | not separately measured |
| Taiga UI | Taiga UI — design reference only (DOM library) | min 24, target 34 | same JNI native path | same native primitives, Taiga branding | built/installed/launched, `showcase-taiga.png`, WebViews=0 | Form primitives not implemented | not separately measured |
| Spartan UI | Spartan UI — design reference only (DOM library) | min 24, target 34 | same JNI native path | same native primitives, Spartan branding | built/installed/launched, `showcase-spartan.png`, WebViews=0 | Tailwind classes have no native meaning | not separately measured |

**Verdict for all five:** `Native reimplementation required`. The third-party library itself was not installed or executed in the APK. The visual language is a reference only.

## Phase 2 — ecosystem showcases

| Feature / showcase | Library + version tested | Android API | Renderer / backend | Native implementation | Tests + execution evidence | Known limitations | Perf |
|---|---|---|---|---|---|---|---|
| Data grid | AG Grid — design reference only (DOM/JS) | min 24, target 34 | `RecyclerView` + `EmployeeAdapter` ViewHolder recycling; Rust JNI bridge online | 10,000 deterministic rows, stable IDs, live recycle counter | APK built/installed/launched; 6 swipes → **177 views recycled**, top position 185; `showcase-ecosystem-grid.png`; `WebViews: 0`; report present | Read-only: no sort/filter/edit/resize/group/pin; no FPS benchmark yet | 529 ms process, 44,289 KB PSS, 20.2 MB APK |
| Charts | Apache ECharts / ng2-charts — design reference only | min 24, target 34 | Android `Canvas` + `Paint` custom Views; Rust JNI bridge alive | `BarChartView`, `LineChartView`, `PieChartView` (bar/line/donut), touch hit-testing tooltip | built/installed/launched; all 3 tabs + bar/line/donut taps produced tooltips (`Q2 '26 33.2 M USD`, `Thu 71 K sessions`, `Organic 28 %`); `showcase-ecosystem-charts.png`; `WebViews: 0`; report present | Static Kotlin data, no zoom/animation/export, no full ECharts option schema | 558 ms process, 42,206 KB PSS, 20.2 MB APK |
| State + i18n + forms + animation | SignalStore / Transloco / Formly / GSAP — **concepts only**, libraries not bundled | min 24, target 34 | Native Android widgets; Rust JNI bridge initialized at startup | observable state holder, EN/UR string catalog, EditText validation, `ObjectAnimator` | built/installed/launched; `showcase-ecosystem-core.png`; no FATAL EXCEPTION; `WebViews: 0`; report present | Locale switch updates headers/hints, not already-created buttons; not unchanged library compatibility | 43,202 KB PSS (process timing sample flagged as artifact in BENCHMARKS.md) |
| Rich text | Tiptap / CKEditor — design reference only | min 24, target 34 | Android `EditText` + `SpannableString` real spans; Rust JNI bridge alive | Bold/Italic/Underline/Bullet/Color/Size/Link/Undo toolbar | built/installed/launched; `showcase-ecosystem-richtext.png` shows native spans; `WebViews: 0`; report present | No HTML import/export, no collaboration/images; no benchmark | not separately measured |
| Calendar | FullCalendar — design reference only | min 24, target 34 | Android custom `View` + `Canvas`/`Paint`; Rust JNI bridge alive | 7-column month grid, today/selected highlights, event dots, tap-to-select, prev/next | built/installed/launched; Next→Dec 2026, Prev→Nov 2026, tap day 6 → details panel; `showcase-ecosystem-calendar.png`; `WebViews: 0`; report present | Month view only; no week/day/agenda, no drag-drop, no calendar sync | not separately measured |
| Build tools | Nx / Angular CLI / Playwright / Vitest / Jest | n/a | read-only Node validator | `scripts/validate-build-tools.mjs` | `node scripts/validate-build-tools.mjs` → **25 PASS / 0 FAIL / 6 INFO** | Nx and Jest not installed; validator not yet in CI | n/a |
| Ecosystem matrix (11 categories) | see `docs/ECOSYSTEM-COMPATIBILITY.md` | n/a | documentation | honest status labels only | no library installed or run; no fabricated results | many categories `Not yet tested` / `Native reimplementation required` | n/a |

## Phase 3 — reusable infrastructure

| Package | Actual content | Evidence | Limitations |
|---|---|---|---|
| `@angnative/native-ui-core` | 8 PlatformOp callbacks, pluggable `viewCreator`, `NativeUiTheme`, `StyleCodec`, `PlatformOp` JSON decoder | `assembleRelease` BUILD SUCCESSFUL, AAR 40,554 bytes; `test` BUILD SUCCESSFUL, 6 tests / 0 failures | No Rust libs/JNI glue/JS runtime; main-thread callbacks; no instrumentation tests |

## Claims explicitly NOT made

- No third-party UI library was installed or executed inside an APK.
- No WebView/Capacitor/embedded browser exists in any showcase (verified via `dumpsys meminfo` `WebViews: 0` and source scans).
- No frame-pacing, CPU, warm-start, physical-device or WebView-comparison benchmark has been run.
- iOS was not built or tested (no macOS/Xcode in this environment).
- Native shells have no embedded JS runtime executing an Angular bundle; see `docs/EVENT-BRIDGE.md`.

