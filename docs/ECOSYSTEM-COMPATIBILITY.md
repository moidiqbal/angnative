# Ecosystem Compatibility — Phase 2 Categories

**Status: 2026-11-21.** Assessment of the 11 Phase 2 ecosystem categories from
the mission brief. This is a **reasoned, evidence-backed assessment**, not a
record of executed integration tests: no third-party library listed below was
installed, built, or run as part of this task. Where a status looks optimistic,
the caveat underneath explains exactly what is and is not proven.

## Evidence and caveats (read first)

- **Native shells currently have NO embedded JS runtime that can execute an
  Angular bundle.** The host engine has QuickJS scaffolding (`runtime.rs`), and
  the FFI event bridge has a passing Rust regression test, but the runtime
  bridge, bundle loading, and platform click wiring are **not done** — see
  [EVENT-BRIDGE.md](EVENT-BRIDGE.md), steps 1–3, and the `no ngc pipeline exists
  yet` note. Consequently, **pure-TS Angular libraries (state, i18n, RxJS,
  reactive forms, etc.) cannot execute unchanged on the native path today**,
  even when they are DOM-free by design.
- **The web adapter (`@angnative/web`) runs in a real browser DOM** and can run
  these libraries. Most "compatible" statuses below describe that path. See
  [COMPATIBILITY.md](COMPATIBILITY.md) for the DOM-vs-native split.
- **Native shells have no DOM, no CSS, and no browser**; the native renderer
  maps `na-*` primitives to `TextView` / UIKit / `wgpu` rectangles. No DOM shim
  is shipped. DOM/contenteditable/canvas libraries therefore need a native port.
- Status labels used, exactly: `Directly compatible`, `Compatible with
  configuration`, `Compatible through native adapter`, `Native reimplementation
  required`, `Unsupported`, `Not yet tested`.

## Matrix

| # | Category | Status | Why |
|---|---|---|---|
| 1 | State management (NgRx, SignalStore, NGXS, Elf) | **Compatible with configuration** | Signals/stores/observables are renderer-agnostic and DOM-free; [COMPATIBILITY.md](COMPATIBILITY.md) marks NgRx SignalStore ✅. Runs unchanged on the web adapter. Native shell execution is blocked on the missing JS runtime. NGXS and Elf were not individually tested. |
| 2 | Data grid (AG Grid) | **Native reimplementation required** | AG Grid is DOM- and virtualization-heavy; [COMPATIBILITY.md](COMPATIBILITY.md) lists `na-data-grid` as a planned native replacement. Web adapter works. |
| 3 | Dynamic forms (Formly / Reactive Forms) | **Compatible with configuration** | Reactive Forms is pure TS / renderer-agnostic and works on the web adapter. Formly needs native field-type mappings for the native path (not implemented). Native shell execution still needs the JS runtime. |
| 4 | Charts (ECharts / ng2-charts / AG Charts) | **Native reimplementation required** | DOM/canvas renderers; [COMPATIBILITY.md](COMPATIBILITY.md) marks ngx-echarts ❌ native and lists `na-chart` (GPU-backed) on the wishlist. Web adapter expected to work; not tested here. |
| 5 | Animation (GSAP / Angular animations / Motion One) | **Native reimplementation required** | GSAP is DOM-only per [COMPATIBILITY.md](COMPATIBILITY.md); Angular animations also depend on DOM-style element mutation. Use native animators on the native path. GSAP works on the web adapter. Motion One not tested. |
| 6 | i18n (Transloco / ngx-translate) | **Compatible with configuration** | Pure TS, no DOM; [COMPATIBILITY.md](COMPATIBILITY.md) marks Transloco ✅. Web adapter runs it. Native shell execution is still blocked on the missing JS runtime, so the ✅ for native is a design expectation, not a demonstrated run. ngx-translate not separately tested. |
| 7 | API / async data (RxJS / TanStack Query) | **Compatible with configuration** | RxJS observables are renderer-agnostic and already underpin Angular core; [COMPATIBILITY.md](COMPATIBILITY.md) marks the pair ✅. TanStack Query is not installed in this repo and was not tested. Native path needs the JS runtime. |
| 8 | Component primitives (Angular CDK / Angular Aria) | **Compatible with configuration** | [COMPATIBILITY.md](COMPATIBILITY.md): CDK behaviours need native equivalents; web adapter works. Angular Aria was not separately tested. |
| 9 | Rich text (Tiptap / CKEditor) | **Native reimplementation required** | Tiptap is contenteditable/DOM-only. A native equivalent is built and verified in `showcases/ecosystem-richtext` (Android `EditText` + real Spannable spans, WebViews=0). It is NOT unchanged Tiptap compatibility. Web adapter works. CKEditor not tested. |
| 10 | Calendar (FullCalendar) | **Native reimplementation required** | DOM-based. A native equivalent is built and verified in `showcases/ecosystem-calendar` (Android Canvas month grid, WebViews=0). NOT unchanged FullCalendar compatibility. Web adapter works. |
| 11 | Microfrontends (Native Federation / Module Federation) | **Not yet tested** | Architecture-dependent; [COMPATIBILITY.md](COMPATIBILITY.md) says "not applicable to native path", web adapter only. No federation setup exists in this repo, so no integration test was run. |

## Interpreting the matrix

- **"Compatible with configuration"** here means: runs on the web adapter, or
  is DOM-free by design, with the explicit caveat that native-shell execution
  is not available until the embedded JS runtime work in
  [EVENT-BRIDGE.md](EVENT-BRIDGE.md) is complete.
- **"Native reimplementation required"** means the library's rendering model
  is DOM/canvas/contenteditable-based; the web adapter can host it, but a
  native AngNative primitive would have to be built for native shells. The
  current wishlist (`na-data-grid`, `na-chart`, `na-calendar`, `na-rich-text`)
  is tracked in [COMPATIBILITY.md](COMPATIBILITY.md) section 4.
- **"Not yet tested"** means there is no integration or test in the repo to
  back a stronger claim; treat it as unknown, not as a failure.

Toolchain facts behind this assessment are validated by
`scripts/validate-build-tools.mjs` — see [BUILD-TOOLS-VALIDATION.md](BUILD-TOOLS-VALIDATION.md).
Re-run that script after any dependency change; re-assess this matrix after the
native runtime lands, since several statuses are expected to improve at that
point.
