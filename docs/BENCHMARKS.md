# Benchmarks — Phase 6 (initial, honest)

**Device:** Android emulator `sdk_gphone64_x86_64`, API 34, x86_64, 1080×2400 @ 420 dpi
**Method:** `scripts/measure-showcase.ps1` — force-stop, `am start`, poll `pidof`, settle 3 s, `dumpsys meminfo`
**Date:** 2026-11-21
**Sample count:** 1 per app (not statistically significant)
**Caveat:** these are cold-start *process appearance* timings on an emulator, not frame-pacing benchmarks. The grid showcase scroll test (177 ViewHolder recycles over 6 swipes) is a separate interaction observation.

| Showcase | APK size | cold process (ms) | cold total (ms) | PSS (KB) | WebViews |
|---|---:|---:|---:|---:|---:|
| ecosystem-grid (10k rows RecyclerView) | 20,204,279 | 529 | 3,541 | 44,289 | 0 |
| ecosystem-charts (native Canvas) | 20,216,291 | 558 | 3,565 | 42,206 | 0 |
| ecosystem-core (state/i18n/forms/animation) | 20,184,398 | 16,593* | 19,600* | 43,202* | 0* |

\* ecosystem-core's first two samples showed ~16 s `pidof` detection time on a busy shared emulator; a manual re-launch showed the process appearing within ~0.4 s of `am start` and a healthy PSS of 43 MB. Treat the 16 s figure as a measurement artifact of the polling method under emulator load, not a real startup regression. Re-run with multiple samples on a quiet emulator before drawing conclusions.

Raw JSON: `docs/benchmarks/*.json`.

## Not yet measured (Phase 6 remaining)
- Frame pacing / dropped frames — **measured on emulator** after restarting the AVD and
  confirming each app was `topResumedActivity`. Raw data: `docs/benchmarks/frame-pacing.json`.
  The earlier "2 frames / 100% janky" sample was invalid (background app, offline device) and
  was discarded. Current single-burst results on **SwiftShader software rendering**:

| Showcase | Frames | Janky % | p50 | p95 |
|---|---:|---:|---:|---:|
| ecosystem-grid (10k RecyclerView) | 186 | 16.13% | 20 ms | 48 ms |
| ecosystem-charts (Canvas) | 70 | 25.71% | 16 ms | 81 ms |
| ecosystem-richtext (Spannable editor) | 23 | 73.91% | 34 ms | 150 ms |
| ecosystem-calendar (Canvas month grid) | 14 | 64.29% | 42 ms | 700 ms |

  **Caveat:** software-rasterized emulator, one burst per app, low frame counts for
  richtext/calendar. These are NOT production performance figures.

### Emulator multi-sample (2026-11-23) — DONE
`docs/benchmarks/emulator-multisample.json` +
`docs/evidence/ANDROID-BENCHMARKS-MULTISAMPLE.md`: 3 swipe bursts per app with
`gfxinfo` framestats and instantaneous `top` CPU samples. Averages: grid 127.3
frames / 48.96% jank / 42.7 ms p50 / 93 ms p95; charts 7 frames, richtext 1 frame,
calendar 8 frames — those low counts make percentiles unstable. CPU samples are
frequently `0.0` and are not sustained utilization.

### Emulator sustained load (2026-11-25) — DONE
`docs/benchmarks/sustained/emulator-20261125-143058.json` +
`docs/evidence/ANDROID-SUSTAINED-LOAD.md`: all 10 showcases, 5 cold + 5 warm
starts each, then 3 × 30 s alternating swipes. `status=ok`, 30/30 samples valid.
CPU is `/proc/<pid>/stat` interval CPU as percent of one core (`nproc=4`), not
instantaneous `top`.

| Showcase | Cold mean (ms) | Warm mean (ms) | Frames | Janky % | p50 | p95 | proc CPU % | ΔPSS (KB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| material | 2100.8 | 537.2 | 0 | — | — | — | 4.61 | -37.0 |
| primeng | 2533.6 | 759.6 | 0 | — | — | — | 5.29 | -31.3 |
| ngzorro | 2486.0 | 847.0 | 0 | — | — | — | 5.33 | -37.3 |
| taiga | 2398.8 | 817.0 | 0 | — | — | — | 4.98 | -28.7 |
| spartan | 1877.2 | 569.4 | 0 | — | — | — | 5.12 | -45.7 |
| ecosystem-grid | 2282.2 | 662.0 | 539.0 | 48.40 | 44.0 | 71.7 | 31.02 | 1309.7 |
| ecosystem-charts | 2503.8 | 713.2 | 25.7 | 96.29 | 36.0 | 63.7 | 7.07 | 130.7 |
| ecosystem-core | 2801.0 | 527.4 | 0 | — | — | — | 3.87 | -134.7 |
| ecosystem-richtext | 2069.4 | 633.0 | 56.3 | 35.30 | 21.7 | 36.3 | 5.29 | -408.3 |
| ecosystem-calendar | 1585.4 | 459.4 | 0 | — | — | — | 3.15 | -118.7 |

Seven apps produced zero new rendered frames after the post-settle `gfxinfo`
reset; their frame metrics are null, not fake `4950 ms` percentiles. All figures
above are **emulator SwiftShader**, not production GPU and not a framework
comparison.

### Physical-device run (OnePlus Open, Android 16) — single burst DONE
`docs/benchmarks/physical/README.md` + `results.json`: all 10 APKs installed/launched,
WebViews=0 on every app. Real-GPU headline: **ecosystem-grid 170 frames, 1.76% jank,
6 ms p50 / 20 ms p95** vs emulator 16.13% / 20 ms. Clean screenshots for all 10 apps are kept in `docs/benchmarks/physical/*.png`
(status bar + navbar cropped, strict foreground guard so notification shades are
never captured). Physical multi-sample, CPU and warm-start are still pending:
`b6640c8f` was not connected on 2026-11-23, so no new physical claim is made.
- Sustained CPU, warm-start and multi-sample averages on the physical device (emulator results now retained; OnePlus offline)
- WebView comparison builds (deliberately not built for native showcases)

Do not present these numbers as a NativeScript/Flutter comparison — no equivalent builds exist in this repo.
