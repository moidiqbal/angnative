# Angular Library Compatibility — AngNative

**Status: 2026-11-21.** This document answers the community question:
"Do third-party Angular libraries that reach for the DOM work on AngNative?"

## Short answer

| Path | DOM libraries | Why |
|---|---|---|
| **Native shells** (Android / iOS / Desktop) | ❌ No | The native renderer only understands `na-*` primitives and maps them to `TextView` / `UIKit` / `wgpu` rectangles. There is no DOM, no CSS, no browser. |
| **Web adapter** (`@angnative/web`) | ✅ Yes | It runs in a real browser DOM — Angular Material, PrimeNG, etc. work normally. |

A library that reads/writes `document`, uses CSS selectors, or renders HTML cannot
run on the native path without a **native port** (or a DOM shim, which we do not
ship — it would erase the performance advantage).

## 1. UI component libraries (community ask)

| Library | Native path | Web adapter | Notes |
|---|---|---|---|
| Angular Material | ❌ needs native port | ✅ works | Map `MatButton` → `na-button`, `MatCard` → `na-view`, etc. |
| PrimeNG | ❌ needs native port | ✅ works | Tables/dialogs are DOM-heavy; native `na-data-grid` is the planned replacement |
| NG-ZORRO | ❌ needs native port | ✅ works | Ant Design tokens can be mapped to native styles |
| Taiga UI | ❌ needs native port | ✅ works | Form primitives need native equivalents |
| Spartan UI | ❌ needs native port | ✅ works | Tailwind classes have no meaning on native widgets |

**Showcase APKs:** `apps/showcase/<library>/` — WebView/Capacitor demos built for
comparison. They are **not** native AngNative performance demos; each README says so.

## 2. Other important Angular libraries

| Category | Library | User preference | Native path | Web adapter |
|---|---|---|---|---|
| State management | NgRx SignalStore | ✅ SignalStore | ✅ (signals are renderer-agnostic) | ✅ |
| Data grid | AG Grid | ✅ AG Grid | ⚠️ needs native `na-data-grid` | ✅ |
| Dynamic forms | Formly / Reactive Forms | ✅ Formly | ⚠️ Reactive Forms yes; Formly needs native field types | ✅ |
| Charts | ngx-echarts | ✅ ngx-echarts | ❌ needs native chart renderer | ✅ |
| Animation | GSAP | ✅ GSAP | ❌ DOM-only; use native animators | ✅ |
| i18n | Transloco | ✅ Transloco | ✅ (pure TS) | ✅ |
| Monorepo | Nx / Angular CLI | ✅ Nx | ✅ tooling only | ✅ |
| Async data | RxJS + TanStack Query | ✅ RxJS + targeted Query | ✅ | ✅ |
| Component primitives | Angular CDK / Angular Aria | ✅ CDK | ⚠️ CDK behaviours need native equivalents | ✅ |
| Testing | Playwright + Vitest | ✅ | ✅ (Rust tests + conformance) | ✅ |
| Rich text | Tiptap | ✅ Tiptap | ❌ DOM-only | ✅ |
| Calendar | FullCalendar | ✅ FullCalendar | ❌ needs native calendar | ✅ |
| Microfrontends | Native Federation | architecture-dependent | ❌ not applicable to native path | ✅ |

## 3. What we recommend to the community

- **Native apps:** use `@angnative/common` primitives + `@angnative/core` renderer.
  Design tokens can imitate Material/Cupertino without the DOM libraries.
- **Complex enterprise admin screens (grids, charts, rich text, calendars):**
  run them through `@angnative/web` in the browser today, or sponsor a native port.
- **State, i18n, RxJS, testing:** these work on both paths already.

## 4. Native port wishlist (community-driven)

1. `na-data-grid` — virtualized rows (Android `RecyclerView`, iOS `UICollectionView`)
2. `na-chart` — GPU-backed charts
3. `na-calendar` — native calendar view
4. `na-rich-text` — native text editing
5. Material 3 / Cupertino token packs for `na-*`
