# The Killer Demo — "Existing Angular → Native, Zero Rewrite"

**Status: candidate adoption story, partially demonstrated (2026-11-22).**

This document defines the single demo that would make AngNative's value obvious
to an Angular team, and records honestly how much of it is proven today.

---

## 1. The claim

> Point the AngNative CLI at an **existing standalone Angular application**,
> ship it to Android/iOS/desktop as a real native view tree — **no React
> Native rewrite, no Flutter rewrite, no WebView**.

That is the differentiator versus React Native (new JS/React code), Flutter
(new Dart code), and Capacitor (WebView). The pitch is **reuse of the Angular
codebase and skills**, not a new UI stack.

---

## 2. What is actually proven today (2026-11-22)

| Link in the chain | Evidence |
|---|---|
| Real Angular AOT compilation (`ngc`) | `pnpm compile:aot` — Ivy `ɵɵdefineComponent` verified |
| Self-contained bundle for QuickJS | `pnpm bundle:quickjs` — IIFE, `es2018`, Angular compiler included |
| Angular executes in QuickJS inside a native process | `angular_bootstrap_runs_in_quickjs` (ignored acceptance test) + on-device screenshot `docs/assets/ecosystem-core-angular-in-rust-nodes20.png` |
| Renderer output drives real native nodes | `na_dispatchBatch` → `Host::process_batch_json`; assertion `host.engine().size() > 0`; on-device label `Angular in Rust: nodes=20 (load rc=0)` |
| Native click reaches an Angular handler | acceptance test asserts one listener fires and a new native batch follows |
| Zero WebView | no WebView dependency; embedded rquickjs engine only |
| All 10 Android shells use the unified bridge | `nativeRuntimeBridgeCreate` in each; all `assembleDebug` observed |

So the **core technical wager is demonstrated**: an Angular AOT app can boot in
QuickJS and mutate a native view tree, on an Android emulator.

---

## 3. What is missing before the demo is honest to show publicly

1. **A real existing app as input.** Today the bundle comes from a purpose-built
   `apps/demo-app`. The demo must ingest an unmodified third-party Angular app
   (or a checked-out sample like a Angular Material dashboard) without editing
   its components.
2. **Full Angular surface on native.** Routing, template-driven forms,
   HttpClient via DI, the real animation engine, and i18n markers are not
   verified. Showcase UIs are still imperative native reimplementations.
3. **One-command CLI flow.** `angnative build` bundles, but `angnative run/test/
   inspect` are placeholders; there is no "ship to device" command.
4. **iOS.** Swift bindings still wrap the legacy host; nothing claims iOS
   execution.
5. **E2E test on a shipped shell.** ecosystem-core is the only shell that loads
   the bundle; the other nine keep imperative UIs.

---

## 4. The concrete demo to build

```
# 1. Take an existing Angular app (no edits)
git clone <some-angular-app>
cd some-angular-app

# 2. One command produces a native-ready bundle
angnative build --entry src/main.ts

# 3. One command ships it to a device
angnative run --platform android

# Expected observable result:
#   - app bootstraps (verifiable today)
#   - native view tree mirrors the Angular template (verifiable today)
#   - taps update the UI through Angular handlers (verifiable today)
#   - navigation between routes works (NOT verified today)
```

**Acceptance criteria for calling this demo "real":**
- zero source edits to the upstream app,
- route change + form submit + HTTP response all produce native mutations,
- same bundle runs on Android and desktop,
- CI job reproduces all of it from a clean checkout.

---

## 5. Honest positioning until then

Do **not** market "any Angular app runs natively". The defensible statement is:

> AngNative demonstrates that an Angular AOT component tree can execute inside
> an embedded QuickJS engine and drive a native view tree with no WebView.
> We are closing the remaining gaps (routing, forms, HTTP, animations, iOS)
> before claiming general framework compatibility.

This is the adoption story to finish — it is currently a roadmap item, not a
shipped product claim.
