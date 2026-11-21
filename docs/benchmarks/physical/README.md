# Physical-device benchmark — OnePlus Open (CPH2551, Android 16)

**Date:** 2026-11-21
**Device:** OnePlus Open, model `CPH2551` (`OP5973L1`), Android 16
**Displays detected:** 2268×2440 (unfolded, 120 Hz) and 1116×2484 (cover, 120 Hz)
**Tool:** `scripts/verify-oneplus.ps1` (adb install → launch → gfxinfo → meminfo)

## Results (real GPU, `dumpsys gfxinfo` after 5 interactions)

| App | Frames | Janky % | p50 | p95 | PSS (KB) | WebViews |
|---|---:|---:|---:|---:|---:|---:|
| material | 31 | 9.68% | 6 ms | 21 ms | 101,698 | 0 |
| primeng | 15 | 6.67% | 10 ms | 200 ms | 101,446 | 0 |
| ngzorro | 6 | 66.67% | 17 ms | 200 ms | 99,018 | 0 |
| taiga | 6 | 33.33% | 15 ms | 200 ms | 99,135 | 0 |
| spartan | 6 | 50.00% | 18 ms | 300 ms | 98,832 | 0 |
| **ecosystem-grid** | **170** | **1.76%** | **6 ms** | **20 ms** | 103,331 | 0 |
| ecosystem-charts | 11 | 45.45% | 15 ms | 250 ms | 102,961 | 0 |
| ecosystem-core* | 165 | 1.82% | ~6 ms | ~20 ms | 104,683 | 0 |
| ecosystem-richtext | 52 | 13.46% | 8 ms | 28 ms | 108,058 | 0 |
| ecosystem-calendar | 16 | 25.00% | 7 ms | 250 ms | 103,118 | 0 |

\* ecosystem-core was re-measured separately after the main run (its row in the script output was blank); foreground confirmed via `mCurrentFocus`.

**Headline:** the 10,000-row virtualized grid rendered **170 frames at 1.76% jank / 6 ms p50** on real hardware — versus 16.13% jank / 20 ms p50 on the SwiftShader emulator. This is the strongest evidence that the Rust-driven native path keeps the UI thread clear.

## Caveats — read before quoting
- Single interaction burst per app; low frame counts (6–16) for some apps make their jank % statistically weak — do not compare those apps against each other.
- All 10 APKs installed and launched successfully; **WebViews: 0** on every app (real device).
- **Screenshots re-captured successfully** — the first capture attempt produced black frames (display OFF / notification shade). Re-capture with screen ON + SystemUI notification shade dismissed produced valid PNGs for all 10 apps: `docs/benchmarks/physical/*-valid.png` (script `scripts/capture-oneplus-shots.ps1`). Verified visually: `ecosystem-grid-valid.png` shows 10,000 rows with 771 recycled views and a readable table on the unfolded display; `ecosystem-charts-valid.png` shows the native Canvas donut chart with legend. Both show `WebViews: 0` in-app. On a foldable, `adb exec-out screencap -p` prepends a "Multiple displays" warning to stdout — the capture script strips everything before the PNG magic bytes (`89 50 4E 47`).
- The USB connection dropped repeatedly during the first run; gfxinfo numbers above were captured before each drop and are valid for the apps shown.
- `results.json` is the machine-readable copy of this table.

## To re-capture screenshots later
1. Reconnect USB, accept "Allow USB debugging" on the phone.
2. `adb -s <serial> shell input keyevent KEYCODE_WAKEUP`
3. Confirm `dumpsys power | grep mWakefulness` = Awake and `dumpsys display` shows `Display State=ON`.
4. Dismiss notification shade (`input keyevent 3`).
5. Launch app, then `adb -s <serial> exec-out screencap -p` and verify PNG magic bytes `89 50 4E 47`.
6. For foldable-specific proof, capture both `-d <display-id>` values from `dumpsys SurfaceFlinger --display-id`.
