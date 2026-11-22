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
| Angular Forms (`FormControl`, `FormGroup`, `Validators`) in QuickJS | Implemented (Phase 1, tested) | `pnpm bundle:forms` produces a self-contained bundle exposing real `@angular/forms`; `angular_forms_work_in_quickjs` verifies value/validity on controls and groups. Template-driven `<form>`/`ngModel` bindings still need a full demo to verify. |
| Angular i18n (`$localize` translations) in QuickJS | Implemented (Phase 1, tested) | Shim in `quickjs_shims.rs` supports `$localize` tagged templates + `loadTranslations`; `test_localize_i18n_translations` verifies a registered translation with interpolation. AOT compilation of i18n markers in real app templates not yet verified. |
| Angular Router in QuickJS | Implemented & verified (tested) | Real AOT router app (`apps/demo-app/src/router/main.router.ts`, `tsconfig.aot.router.json`) bundles and bootstraps; `angular_router_bootstraps_with_outlet` verifies the outlet anchor and initial route (`HOME ROUTE`) are built in the native tree, and subsequent imperative navigation (`navigateByUrl('/about')`) successfully swaps route components and produces native mutations (`ABOUT ROUTE`). Cycle-free `AbortController`/`AbortSignal` and `URL` shims plus renderer parent-lookup fix resolve blocker F7. |
| Angular Animations API in QuickJS | Loaded + no-op driver tested | Real `@angular/animations` bundle loads; a no-op `Element.animate()` driver (with `finished` Promise) is verified by `test_web_animations_noop_driver`. Integration with Angular's animation engine / RendererFactory2 is not verified. |
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
