# Angular counter layout — host-level non-overlap evidence

**Date:** 2026-11-23
**Scope:** Host-level layout proof for the regenerated flagship AOT bundle.
This is **not** an on-device screenshot or video.

## Command

```text
pnpm --filter @angnative/core build
pnpm compile:aot && pnpm bundle:quickjs
cargo test -p angnative-runtime-bridge angular_counter_layout_has_non_overlapping_frames -- --ignored --nocapture
```

Result: **1 passed; 0 failed** (ignored-suite test retained in
`crates/angnative-runtime-bridge/src/lib.rs`).

## Retained rects

The test loads the real `dist/aot/quickjs-bundle.js`, snapshots the host tree,
runs `Host::run_layout_json`, and converts parent-relative frames to
screen-space rectangles. Observed values:

| Control | Rect (x, y, width, height) |
| --- | --- |
| `Count: 0` | `(40, 150, 120, 28)` |
| `−` button | `(40, 190, 48, 36)` |
| `reset` button | `(96, 190, 48, 36)` |
| `+` button | `(152, 190, 48, 36)` |

All four have positive size and are pairwise non-overlapping. The three button
origins increase left-to-right (`40 < 96 < 152`), and the counter label sits on
a different row (`y=150` vs `y=190`).

## Not verified here

- No emulator/physical-device run was available. The Android instrumentation
  test `AngularCounterLayoutTest.angularCounterViewsDoNotOverlap` was added and
  its Kotlin compiled, but the test itself is **not verified locally**.
- Physical OnePlus `b6640c8f` remains **not verified**.
- GitHub Actions `android-e2e` has never executed (upstream lockfile failure);
  no claim that CI protects this fix is made.

See `docs/plans/2026-11-23-angular-counter-layout-fix-design.md`.
