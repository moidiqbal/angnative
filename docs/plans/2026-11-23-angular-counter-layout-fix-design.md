# Angular counter layout overlap fix — design

**Date:** 2026-11-23
**Status:** Validated with user; ready for implementation
**Scope:** Flagship `showcases/ecosystem-core` presentation only

## Problem

The flagship Angular counter is functionally correct (tap → Angular listener →
`Count: 1`) but visually broken: `Count: 1`, `reset`, `-` and `+` all share the
same top-left origin. Retained emulator XML shows identical `[92,2100]` origins
for all four controls, while the native demo buttons below have distinct bounds.

There are two independent causes:

1. **Style-key mismatch.** Angular AOT emits kebab keys (`flex-direction`,
   `font-size`, `font-weight`, `margin-top`). `packages/core/src/renderer.ts`
   records the raw key in `SetStyle`, but Rust `angnative-host-layout` only
   accepts camelCase keys and rejects the pass with
   `Unknown style property: flex-direction`. `JniBridge.runLayout` returns an
   error JSON, and `NativeViewFactory.applyLayoutResult` silently ignores it,
   so no frame is applied.
2. **No intrinsic text measurement on the FFI path.** `buildLayoutRequest`
   drops `text_content`, and the demo leaves have no explicit `width`/`height`.
   Even with canonical keys, text and buttons would measure 0×0 and collapse.

This round fixes both for the flagship without changing the JNI/FFI schema.

## Decisions

- **Canonicalize at the source.** `packages/core/src/renderer.ts` resolves the
  kebab name through the shared style table and emits the canonical camelCase
  key in both `element.setStyle` and `SetStyle.key`, matching the web renderer's
  existing normalization.
- **Deterministic demo sizing.** Add explicit `width`/`height` to every
  `na-text`/`na-button` leaf in `apps/demo-app`. This is a demo-only
  accommodation for the still-stubbed generic intrinsic measurement; it is not
  a claim that arbitrary Angular text auto-sizes natively.
- **Make layout failure loud.** `EcosystemCoreActivity` logs the layout JSON
  and `lastError()`; `NativeViewFactory.applyLayoutResult` returns `false` on an
  error or missing `nodes` instead of silently doing nothing.
- **Regenerate the real asset.** Rebuild `@angnative/core` first
  (`pnpm --filter @angnative/core build`), then `pnpm compile:aot &&
  pnpm bundle:quickjs` rebuilds the git-tracked
  `showcases/ecosystem-core/app/src/main/assets/angnative-bundle.js`.
  Skipping the package rebuild uses stale `dist` and leaves kebab keys in the
  bundle.
- **No structural relayout this round.** The count change only sets text; it
  does not change the tree. Re-running layout after arbitrary Angular batches
  is a separate concern.
- **Out of scope and explicitly deferred:** GitHub Actions `android-e2e` (it has
  never executed on Actions because upstream jobs fail on an outdated pnpm
  lockfile), one-command `angnative build/run`, the other nine shells, DOM-heavy
  library docs, and `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`.
- No WebView, no iOS/macOS runtime claim. If event wording appears, use exactly
  “one delivery per event (canonical JS thread-local path + legacy FFI fallback)”.

## Files

- `packages/core/src/renderer.ts`, `packages/core/src/renderer.spec.ts`
- `apps/demo-app/src/counter.component.ts`, `apps/demo-app/src/app.component.ts`
- `crates/angnative-runtime-bridge/src/lib.rs`, `Cargo.toml` (`serde_json` dev-dep)
- `showcases/ecosystem-core/app/src/main/java/com/angnative/app/EcosystemCoreActivity.kt`
- `showcases/ecosystem-core/app/src/main/java/com/angnative/app/NativeViewFactory.kt`
- `showcases/ecosystem-core/app/src/androidTest/java/com/angnative/app/AngularCounterLayoutTest.kt`
- `showcases/ecosystem-core/app/src/main/assets/angnative-bundle.js` (generated)
- `docs/evidence/ANGULAR-COUNTER-LAYOUT.md` (new, small)

## Tests and evidence

1. **TS:** kebab style input emits canonical camelCase `SetStyle` keys.
2. **Host Rust (ignored):** load the real regenerated bundle, snapshot the host
   tree, run `Host::run_layout_json`, assert no `error` and four positive-size,
   pairwise non-overlapping rects for `Count`, `reset`, `-`, `+`. This runs in
   the existing required ignored-suite command.
3. **Android instrumentation:** boot the real bundle, assert layout JSON has
   `nodes` and the four controls' `FrameLayout.LayoutParams` are disjoint.
   Added but **not verified locally** — no emulator/device is available.

`docs/evidence/ANGULAR-COUNTER-LAYOUT.md` retains the host test command and the
four rects, labelled as host-level layout proof, not an on-device screenshot.

## Honest caveats

- Generic intrinsic text measurement remains a stub; explicit sizes are
  demo-only.
- Style `px` is applied as raw device px while layout frames are scaled by
  density; this can leave small offsets but is not the overlap cause. Deferred.
- No device screenshot/video can be produced in this environment. Physical
  OnePlus `b6640c8f` stays **not verified**.
- The retained CI finding is configuration/intent only: `android-e2e` was
  skipped in all 26 runs where it existed. No claim that CI protects this fix.

## Validation and delivery

Full gate only at the end, in order:

1. `cargo test --workspace`
2. `cargo test -p angnative-runtime-bridge -- --ignored`
3. `pnpm test`
4. `python scripts/check-ffi-declarations.py`

No commit/push if any step fails. After a clean pass, commit with
`fix(showcase): apply canonical styles and explicit demo sizing`, then push.
