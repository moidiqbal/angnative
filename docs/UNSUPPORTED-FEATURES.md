# Unsupported / Not-Yet-Tested Feature Register

**Date:** 2026-11-21. Maintained as part of Phase 7 deliverables.
**Rule:** "not yet tested" is never written as "works".

## Native renderer (all shells)

| Feature | Status | Reason / next step |
|---|---|---|
| Third-party DOM libraries (Material, PrimeNG, NG-ZORRO, Taiga, Spartan) | Unsupported unchanged | Renderer only understands `na-*` → native widgets. Need native ports or adapters. |
| AG Grid sorting / filtering / editing / pinning | Not implemented | Grid POC is read-only (`showcases/ecosystem-grid`). |
| ECharts zoom / animation / full option schema | Not implemented | Native Canvas charts POC only. |
| Rich text editing (Tiptap/CKEditor) | Not implemented | Needs native `EditText` spans editor. |
| Calendar (FullCalendar) | Not implemented | Needs native month/week view. |
| Microfrontends (Native Federation) | Not yet tested | Requires JS runtime, absent on native shells. |
| Angular bundle execution on native | Not implemented | QuickJS bridge scaffolding exists (`docs/EVENT-BRIDGE.md`), runtime integration pending. |
| Angular `(click)` listener round trip on native | Partially implemented | FFI event bridge + QuickJS API exist; platform buttons still dispatch imperative Rust batches. |
| iOS build / simulator run | Not tested | No macOS/Xcode in this environment. Code ready (`docs/IOS-BUILD.md`). |
| Frame pacing / CPU benchmark | Not measured | `scripts/measure-frames.ps1` exists but the shared emulator went offline during collection; no frame stats captured. `docs/BENCHMARKS.md` has startup/memory only. |

## Web adapter (`@angnative/web`)

| Feature | Status | Reason |
|---|---|---|
| AOT/ngc production build | Not implemented | Uses JIT `@angular/compiler`; bundle ~2.3 MB dev. |
| Web size-budget gate | Not implemented | JS gate covers fixtures only. |
| SSR / hydration | Not implemented | Out of scope. |

## General

| Feature | Status |
|---|---|
| Native adapter packages (`material-adapter`, `primeng-adapter`, etc.) | Deliberately not created — no empty placeholders rule; each needs real code first |
| Zero-copy transport | Planned — current wire format is JSON |
| Binary mutation protocol | Planned — current uses named string kinds |
| Physical-device Android verification | Not done — emulator only |
