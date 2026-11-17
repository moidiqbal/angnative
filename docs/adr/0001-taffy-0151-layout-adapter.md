# ADR 0001: Isolate layout behind the Taffy 0.15.1 adapter

- Status: Accepted
- Date: 2026-11-17

## Context

`angnative-host-layout` needs production Flexbox behavior, CSS-subset style
mapping, intrinsic host measurement, and safe updates as the retained AngNative
tree changes. The previous implementation did not compile against the resolved
Taffy crate and its `TextMeasurer` was never called. Taffy 0.15.1 is pinned in
the workspace and lockfile.

## Decision

- Keep Taffy as the layout engine; do not add an AngNative CSS layout engine.
- Keep Taffy-specific imports, node IDs, tree creation, measurement callback,
  and layout extraction in `taffy_adapter.rs`.
- Translate AngNative property maps at the boundary using Taffy's typed
  `Dimension`, `LengthPercentageAuto`, `LengthPercentage`, and `Rect` values.
- Keep `LayoutNodeInput`, `LayoutResult`, `LayoutNodeRect`, `TextMeasurer`, and
  the existing `LayoutEngine::layout` entry point source-compatible. Add
  `layout_with_intrinsic_content` for text and image content metadata.
- Give text and image intrinsic sizing explicit host callbacks. The legacy
  constructor uses a deterministic text measurer and a zero-size image
  fallback until a host supplies an image measurer.
- Treat each input snapshot as authoritative: clear the adapter tree, validate
  the graph, rebuild a safe AngNative-ID-to-Taffy-ID map, and compute. This
  prevents stale node contexts or cached geometry after insert, remove,
  re-parent, or style changes. A future optimization may retain nodes only if
  it preserves this invalidation behavior.

## Consequences

- Taffy API changes are contained to the adapter and style translation seam;
  host-view and the command protocol consume AngNative IDs and rectangles.
- Every pass rebuilds the Taffy tree. This favors correctness and bounded
  lifecycle behavior over incremental layout performance until a comparable
  baseline exists.
- The pre-change host-layout did not compile, so there is no working runtime
  baseline to compare benchmark numbers against. A before/after performance
  claim would be misleading; establish a baseline from this verified version
  before optimizing.

## Validation record

Executed on 2026-11-17:

- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --locked` — passed.
- `cargo test --workspace --locked` — passed (54 Rust unit/integration tests
  plus doc tests; host-layout: 18, shared Rust conformance: 14).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `pnpm exec vitest run packages/testing/src/conformance.spec.ts packages/contract/src`
  — passed (95 tests).
- `pnpm run build` and `pnpm exec tsc -b` — passed after the platform package
  dependency/import fixes.
- Full `pnpm test` — passed (171 tests across 13 files). The initial sandboxed
  run could not access the bundler's generated scratch paths; rerunning with
  normal workspace filesystem access passed, confirming that failure was
  environmental. Renderer setup and platform integration failures were fixed.

The passing integration suite does not launch a real Angular application: its
bootstrap assertion only checks that `bootstrapNativeAngularApp` is importable.
An Angular AOT compile of `packages/common` and `apps/demo-app` passed, but
running the emitted `main.js` directly with Node stopped at an unresolved
extensionless ESM import from `@angnative/platform`. Native startup, Rust event
round-trip, and Android execution therefore remain unverified; the app bundling
and Angular linker path must be completed before claiming those milestones.

No working pre-change host-layout baseline exists: the old implementation did
not compile against its resolved dependency. Therefore no comparative
benchmark is recorded; the adapter's full-tree rebuild should be benchmarked
against this verified implementation before incremental caching is optimized.
