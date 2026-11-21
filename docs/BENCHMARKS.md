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
- Frame pacing / dropped frames — `scripts/measure-frames.ps1` was added, but the shared
  emulator repeatedly went `offline`. A direct `dumpsys gfxinfo ... framestats` call did
  return data for `ecosystem-grid`, but only **2 frames** with "100% janky / 950 ms"
  from a non-foreground app on an offline device. That is an invalid sample, not a
  performance result, so **no FPS / jank number is claimed**. Re-run on a stable
  emulator or physical device before publishing frame-pacing figures.
- CPU utilization during scroll
- Warm start, multi-sample averages, physical device
- WebView comparison builds (deliberately not built for native showcases)

Do not present these numbers as a NativeScript/Flutter comparison — no equivalent builds exist in this repo.
