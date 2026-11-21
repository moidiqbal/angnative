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
- **Screenshots: all 10 apps** — clean PNGs under this folder. The capture scripts strip the foldable "Multiple displays" stdout warning, crop the status bar (140 px) and bottom navbar (130 px) to avoid personal information, and only save when `mCurrentFocus` belongs to the target app (a notification shade can never be captured). Verified: `ecosystem-grid.png` shows the 10,000-row virtualized table, `ecosystem-charts.png` the native Canvas donut, `ecosystem-richtext.png` the native Spannable editor, and `ecosystem-calendar.png` the native month grid — all with `WebViews: 0`. The recycle counter in a capture is a live value (51/119/…) and changes per run — not a fixed benchmark number.
- The USB connection dropped repeatedly during the first run; gfxinfo numbers above were captured before each drop and are valid for the apps shown.
- `results.json` is the machine-readable copy of this table.

## To re-capture screenshots later
1. Reconnect USB, accept "Allow USB debugging" on the phone.
2. `adb -s <serial> shell input keyevent KEYCODE_WAKEUP`
3. Confirm `dumpsys power | grep mWakefulness` = Awake and `dumpsys display` shows `Display State=ON`.
4. Collapse notifications (`input keyevent 111`, `cmd statusbar collapse`) and verify `mCurrentFocus` is the target app before saving.
5. Capture, strip stdout warning before PNG magic bytes (`89 50 4E 47`), crop 140 px top + 130 px bottom.
6. Scripts: `scripts/capture-clean-shots.ps1` (all 10) and `scripts/capture-missing-two.ps1` (strict foreground guard for the two that previously caught the shade).
