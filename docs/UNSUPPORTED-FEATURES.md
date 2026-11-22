# Unsupported / Not-Yet-Tested Feature Register

**Date:** 2026-11-21, updated 2026-11-22. Maintained as part of Phase 7 deliverables.
**Rule:** "not yet tested" is never written as "works".

## Native renderer (all shells)

| Feature | Status | Reason / next step |
|---|---|---|
| Third-party DOM libraries (Material, PrimeNG, NG-ZORRO, Taiga, Spartan) | Unsupported unchanged | Renderer only understands `na-*` → native widgets. Need native ports or adapters. |
| AG Grid sorting / filtering / editing / pinning | Not implemented | Grid POC is read-only (`showcases/ecosystem-grid`). |
| ECharts zoom / animation / full option schema | Not implemented | Native Canvas charts POC only. |
| Rich text editing (Tiptap/CKEditor) | Native equivalent built | `showcases/ecosystem-richtext` — native Spannable editor; not unchanged Tiptap compatibility; no HTML import/export, no redo. |
| Calendar (FullCalendar) | Native equivalent built | `showcases/ecosystem-calendar` — native Canvas month grid; month view only, no week/day/agenda, no event sync. |
| Microfrontends (Native Federation) | Not yet tested | Requires JS runtime, absent on native shells. |
| Angular bundle execution on native | Implemented (bootstrap level) | Real AOT bundle runs in QuickJS; `angular_bootstrap_runs_in_quickjs` verified component construction + native `CreateNode` batches. See `docs/CODE-REVIEW-2026-11-22.md`. Full feature parity still absent (below). |
| Angular `(click)` listener round trip on native | Implemented in QuickJS runtime (tested) | Renderer registers via `na_addEventListener`; `na_onEvent` routes Rust dispatch to Angular callback; acceptance test asserts one listener fired + new native batch. Showcase activity UI is still imperative Kotlin, so no on-device tap test of this path. |
| Angular Forms in QuickJS | Implemented & verified (tested) | Reactive forms: `pnpm bundle:forms` + `angular_forms_work_in_quickjs` verify `FormControl`/`FormGroup`/`Validators`. Template-driven: `pnpm compile:aot:forms && pnpm bundle:framework` + `angular_template_driven_forms_round_trip_in_quickjs` verify `FormsModule`/`ngModel` on `<input>`: JS→native property/text mutations and Rust-dispatched native `input` event→model update. `<input>` maps to `na-text-input`; flat `navigator.userAgent` shim added for Angular's DefaultValueAccessor. |
| Angular i18n (`$localize` translations) in QuickJS | Implemented & verified (tested) | `pnpm compile:aot:i18n && pnpm bundle:framework` build `dist/aot/i18n-app-bundle.js` from `<na-text i18n>Hello {{ name }}!</na-text>`; the AOT output contains `$localize`, `ɵɵi18n`, `ɵɵi18nExp` and `ɵɵi18nApply`. `angular_aot_i18n_translated_text_in_native_tree` registers a translation before bundle evaluation and asserts `Assalam-o-Alaikum AngNative!` in the native tree. Root causes fixed: the `$localize` shim used an unbounded replace loop when AOT passes the literal `\uFFFD0\uFFFD` placeholder (now single-pass split/join), and Angular's i18n runtime needs a flat `Node` constants shim. |
| Angular Router in QuickJS | Implemented & verified (tested) | Real AOT router app (`apps/demo-app/src/router/main.router.ts`, `tsconfig.aot.router.json`) bundles and bootstraps; `angular_router_bootstraps_with_outlet` verifies the outlet anchor and initial route (`HOME ROUTE`) are built in the native tree, and subsequent imperative navigation (`navigateByUrl('/about')`) successfully swaps route components and produces native mutations (`ABOUT ROUTE`). Cycle-free `AbortController`/`AbortSignal` and `URL` shims plus renderer parent-lookup fix resolve blocker F7. |
| Angular Animations engine in QuickJS | Implemented & verified (tested, deterministic samples only) | Real `AnimationEngine` + `AnimationRendererFactory` now run over `AngNativeRendererFactory2` via `provideNativeAnimations()`. `pnpm compile:aot:animations && pnpm bundle:framework` builds `dist/aot/animations-app-bundle.js` from an AOT component with `trigger('fade', ...)` and explicit keyframes at offsets `0 / 0.5 / 1`; ignored test `angular_animation_engine_interpolates_native_styles` asserts an intermediate native `SetStyle opacity: 0.5` mutation between start and end, `start,done` trigger callbacks, and a final native `opacity: 1`. Root causes fixed: Angular creates the root renderer with a null host (now a flat synthetic host), the engine requires `node.nodeType === 1` (numeric DOM-compatible `nodeType`, native kind preserved as `nativeNodeType`), `eraseStyles` writes `element.style[...]` (flat style mirror added), the orphan check needs `containsElement(document.body, el)` (native elements are now treated as attached), and `animate('100ms')` alone yields zero keyframes (explicit keyframes used). The native player applies three deterministic samples through flat numeric-id registries — no wall-clock tweening; real-time easing fidelity is **not verified**. The separate no-op `Element.animate()` shim in the QuickJS DOM shim is still tested by `test_web_animations_noop_driver`. |
| Angular HttpClient in QuickJS | Implemented & verified (tested) | Real `@angular/common/http` bundle loads and DI-based `HttpClient.get()` works. `angular_http_platform_loads_in_quickjs` asserts the body from the host fetch handler (`hello-from-native|GET`) and `HttpResponse.status === 200`. Root cause of the former NG0201: `createEnvironmentInjector()` has scope `{'environment'}`, but its plain `Injector.create()` parent had no `root` scope, so `providedIn: 'root'` tokens (`HTTP_FETCH_MAX_RESPONSE_SIZE`, `PendingTasks`, `REQUESTS_CONTRIBUTE_TO_STABILITY`) were unresolvable. Providing `{provide: INJECTOR_SCOPE, useValue: 'root'}` from the same `@angular/core` instance fixes it. |
| iOS build / simulator run | Not tested | No macOS/Xcode in this environment. Code ready (`docs/IOS-BUILD.md`). |
| Frame pacing / CPU benchmark | Emulator-only data captured | `docs/benchmarks/frame-pacing.json` — gfxinfo jank/p50/p95 for 4 showcases on SwiftShader emulator, single burst; physical-device multi-sample and CPU measurement still pending. |

## Web adapter (`@angnative/web`)

| Feature | Status | Reason |
|---|---|---|
| AOT/ngc production build | Implemented and verified | `scripts/compile-aot.mjs` resolves the real Angular compiler-cli `ngc` (the workspace's `npx ngc` is a shadowing 0.1.0 plain-JS emitter). `pnpm run compile:aot` and `node packages/cli/dist/cli.js build` both succeed; `dist/aot/*.js` contains `ɵɵdefineComponent` / template functions. The web adapter's browser demo still uses JIT — migrating it to consume `dist/aot/bundle.js` is a separate task. |
| Web size-budget gate | Not implemented | JS gate covers fixtures only. |
| SSR / hydration | Not implemented | Out of scope. |

## General

| Feature | Status |
|---|---|
| Native adapter packages (`material-adapter`, `primeng-adapter`, etc.) | Deliberately not created — no empty placeholders rule; each needs real code first |
| Zero-copy transport | Planned — current wire format is JSON |
| Binary mutation protocol | Planned — current uses named string kinds |
| Physical-device Android verification | Not done — emulator only |
