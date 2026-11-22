# AngNative vs NativeScript & other cross-platform frameworks

**Date:** 2026-11-21
**Rule:** architectural facts are sourced; performance numbers are shown **only** where
they were actually measured in this repo. No head-to-head benchmark was run against
NativeScript / Flutter / React Native, so **no performance superiority is claimed**.

## TL;DR

| If you want… | Best fit |
|---|---|
| Ship a production Angular app natively **today**, mature ecosystem | **NativeScript** (`@nativescript/angular`) |
| Experiment with a Rust-computed native UI + Taffy layout, zero WebView, MIT | **AngNative** (v0.2.0) |
| One codebase across web + native with DOM libraries | **Capacitor/Ionic** (WebView) or AngNative's `@angnative/web` for web |
| Pixel-identical custom UI across platforms | **Flutter** (owns its pixels — not native widgets) |
| React ecosystem | **React Native** |

## Architecture comparison

| | **AngNative** | **NativeScript 9.1** | **React Native** | **Flutter** | **Capacitor / Ionic** |
|---|---|---|---|---|---|
| UI primitives | Real native widgets via Rust `PlatformOp` → JNI/C-ABI | Real native widgets, direct native API access via metadata bridge | Real native widgets via Fabric/JSI | Skia/Impeller-drawn widgets (**not** platform widgets) | HTML/CSS in a **WebView** |
| Rendering brain | Rust host: mutation shadow tree + **Taffy 0.15.1** layout before UI thread | JS runtime (`V8 14.9` on Android, JavaScriptCore on iOS) drives native views | JS (Hermes/JSC) + Yoga layout | Dart VM + own layout engine | Browser engine |
| WebView in native mode | ❌ none | ❌ none | ❌ none | ❌ none | ✅ **yes** |
| Angular support | Custom `Renderer2` + `na-*` primitives; Angular 22 signals/zoneless | Mature `@nativescript/angular` (since 2014, OpenJS Foundation) | n/a (React) | n/a (Dart) | Angular web stack runs in WebView |
| Does your compiled Angular app run in the native shell today? | ⚠️ **In the embedded runtime, not the shipping shell** — real AOT bundle boots in QuickJS with a working click round trip (7 acceptance tests); shells still render Rust/Kotlin-driven trees (see `EVENT-BRIDGE.md`) | ✅ **Yes** — that is NativeScript's core product | n/a | n/a | ✅ yes (as a web app) |
| Layout computation | Rust/Taffy, off the UI thread | Native platform layout / Yoga-style | Yoga | Flutter engine | Browser layout |
| JS runtime | QuickJS embedded via `angnative-runtime-bridge`; verified in-process, not yet loading bundles in native shells | V8 / JavaScriptCore, production-proven | Hermes / JSC | none (Dart AOT) | browser JS |
| Production AOT | ✅ real Angular `ngc` + esbuild (`dist/aot/`, Ivy verified) | Angular AOT via NativeScript build pipeline | Metro/JS bundling | Dart AOT | web bundling |
| Maturity | 🟡 v0.2.0, new | 🟢 since 2014, OpenJS Foundation, production case studies | 🟢 Meta-backed, huge ecosystem | 🟢 Google-backed | 🟢 widely used |
| License | MIT | Apache-2.0 (OpenJS) | MIT | BSD-style | MIT |

## AngNative's measured numbers (this repo only)

Device: **OnePlus Open, CPH2551, Android 16**, real GPU (details: `benchmarks/physical/README.md`).

| Benchmark | Result |
|---|---|
| 10,000-row virtualized grid | **170 frames, 1.76% jank, 6 ms p50 / 20 ms p95** |
| Same grid on SwiftShader emulator | 16.13% jank, 20 ms p50 |
| WebViews in all 10 showcase APKs | **0** |
| APK size | ~19.2 MB per showcase |
| Test suites | 133 Rust (+7 ignored acceptance, all passing) + 220 JS tests pass |

These are AngNative-only measurements. **We have not built or measured equivalent
NativeScript/Flutter/React Native apps**, so any claim like "faster than X" would be
fabrication. The cross-framework benchmark is a planned task.

## Honest weaknesses vs NativeScript

1. **NativeScript actually runs your Angular app natively today; AngNative does not yet.**
   Our native shells render Rust-driven demo trees. Wiring compiled Angular bundles into
   QuickJS is the next major milestone (`docs/EVENT-BRIDGE.md`).
2. **Ecosystem & tooling.** NativeScript has Vite HMR (9.1), SimDeck, an OpenJS
   foundation, years of plugins and case studies. Our CLI has doctor/create/build/dev,
   but HMR is still a placeholder (`packages/cli/src/dev.ts:175`).
3. **Angular renderer parity.** NativeScript's `@nativescript/angular` is battle-tested
   against real apps; ours passes conformance fixtures and browser tests but has not
   run a production enterprise Angular app.
4. **Community / docs.** NativeScript has Stack Overflow, Discord, StackBlitz examples.
   We have engineering docs and a verification report — growing.

## Honest strengths vs NativeScript

1. **Layout computed in Rust before the UI thread** (Taffy), with measured jank numbers
   on real hardware. NativeScript's layout is driven from the JS runtime.
2. **Tiny, auditable host core** (Rust, `panic=abort`, LTO, 1 MB budget policy) —
   no JS VM required in the native rendering path today.
3. **One renderer for native and DOM** (`@angnative/web` shares the command protocol),
   so the same `na-*` components work in both worlds.
4. **Full public evidence trail**: per-feature compatibility matrix, library versions,
   test commands, benchmarks, ADRs, unsupported-feature register.

## When to choose what

- **Production Angular mobile app now** → NativeScript. It is mature and actually
  executes Angular code on-device. AngNative is not a drop-in replacement yet.
- **Zero-WebView Rust-native experiment / research** → AngNative, with eyes open that
  the Angular-in-native runtime is not connected yet.
- **Capacitor/Ionic** → fine when a WebView is acceptable and you want web libraries
  unchanged; explicitly not a native renderer.
- **Flutter** → choose for custom-designed, pixel-identical UIs (it does not use
  platform widgets, which may or may not be what you want).
- **React Native** → choose for React teams; comparable native-widget philosophy.

## Sources
- NativeScript homepage / 9.1 announcement: https://nativescript.org/ (V8 14.9, Vite 8 HMR, OpenJS Foundation)
- AngNative evidence: `VERIFICATION-REPORT.md`, `COMPATIBILITY-EVIDENCE.md`, `benchmarks/physical/README.md`
