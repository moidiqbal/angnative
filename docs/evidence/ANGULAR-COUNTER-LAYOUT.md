# Angular counter layout — host and physical-device evidence

**Date:** 2026-11-23
**Scope:** Flagship `showcases/ecosystem-core` AOT bundle. This retains both the
host-level layout proof and the physical OnePlus `b6640c8f` on-device proof.
No WebView is involved.

## Commands

Host-level regression:

```text
pnpm --filter @angnative/core build
pnpm compile:aot && pnpm bundle:quickjs
cargo test -p angnative-runtime-bridge angular_counter_layout_has_non_overlapping_frames -- --ignored --nocapture
```

Physical device (from `showcases/ecosystem-core`, after rebuilding staged JNI
staticlibs with `pwsh -File scripts\build-android-native.ps1`):

```powershell
$env:ANDROID_SERIAL='b6640c8f'
.\gradlew.bat connectedDebugAndroidTest --no-daemon
```

## Results

- Host ignored test: **1 passed; 0 failed**.
- OnePlus instrumentation suite: **8 passed; 0 failed; 0 skipped**. Full detail,
  live UI rects, logcat and screenshot are in
  `docs/evidence/ANGULAR-COUNTER-ONEPLUS.md`.

## Retained host rects

The host test loads the real `dist/aot/quickjs-bundle.js`, snapshots the host
tree, runs `Host::run_layout_json`, and converts parent-relative frames to
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

## Honest scope

- Generic intrinsic text measurement remains a stub. Explicit demo sizes and
  `[text]` bindings are demo-only accommodations, not a claim that arbitrary
  Angular text auto-sizes natively.
- Static `ɵɵtext` children receive zero-size frames on Android; the demo avoids
  that path by using the supported `text` input. The generic renderer gap is not
  fixed here.
- The physical OnePlus `b6640c8f` is **verified locally** for this fix; a
  screenshot is retained, no video.
- GitHub Actions `android-e2e` has never executed (upstream lockfile failure);
  no claim that CI protects this fix is made.
- iOS was not run: no Mac/Xcode, Swift compile, Simulator, or device evidence.

See `docs/plans/2026-11-23-angular-counter-layout-fix-design.md` and
`docs/evidence/ANGULAR-COUNTER-ONEPLUS.md`.
