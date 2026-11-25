# Android sustained-load benchmark evidence

**Date:** 2026-11-25
**Plan:** `docs/plans/2026-11-25-sustained-load-benchmark-design.md`

## Device and command

`adb devices -l` at measurement time:

```text
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1
```

Properties retained in the JSON: `sdk_gphone64_x86_64`, Android 14 (API 34),
1080×2400, 4 CPUs, `CLK_TCK=100`, SwiftShader software rendering
(`-gpu swiftshader_indirect`). This is not a physical GPU.

```powershell
pwsh -NoProfile -File scripts/measure-sustained.ps1 -Device emulator-5554 -Samples 3 -DurationSeconds 30 -StartReps 5
```

Raw retained output: `docs/benchmarks/sustained/emulator-20261125-143058.json`
— `status=ok`, 30/30 samples valid, 0 run errors.

An earlier run is retained as
`docs/benchmarks/sustained/emulator-20261125-134644-superseded.json` but is
**not used for claims**. It exposed two parser defects that are fixed in the
harness: `am start -W` sometimes omits `TotalTime` on warm `ecosystem-core`
launches, and `gfxinfo` reports a synthetic `4950 ms` percentile when zero
frames were rendered.

## Method

For each of the 10 showcase APKs:

1. 5 cold reps: force-stop, `am start -W`, `pidof` confirmation, foreground guard.
2. 5 warm reps: HOME, confirm the same PID survives, relaunch, foreground guard.
3. 3 samples × 30 s of alternating swipes (`x=540`, `y=1872 ↔ 672`, 250 ms).
   Actual swipe counts are recorded per sample (42–57 swipes).
4. `gfxinfo reset` **after** a 3 s settle so startup frames are excluded.
5. `/proc/<pid>/stat` CPU deltas every 1 s (authoritative), `top` samples for
   continuity, `dumpsys meminfo` every 5 s plus t=0 and t=end.
6. Foreground checked at sample start and end; aggregates use only valid samples.

## Start distribution (5 cold + 5 warm per app)

| App | Cold mean (ms) | Cold p95 (ms) | Warm mean (ms) | Warm p95 (ms) |
|---|---:|---:|---:|---:|
| material | 2100.8 | 2308 | 537.2 | 596 |
| primeng | 2533.6 | 2855 | 759.6 | 870 |
| ngzorro | 2486.0 | 2769 | 847.0 | 1138 |
| taiga | 2398.8 | 2585 | 817.0 | 907 |
| spartan | 1877.2 | 2026 | 569.4 | 649 |
| ecosystem-grid | 2282.2 | 2470 | 662.0 | 735 |
| ecosystem-charts | 2503.8 | 2758 | 713.2 | 881 |
| ecosystem-core | 2801.0 | 3031 | 527.4 | 628 |
| ecosystem-richtext | 2069.4 | 2298 | 633.0 | 868 |
| ecosystem-calendar | 1585.4 | 1648 | 459.4 | 482 |

`process_ms` is wall time to `am start -W` plus `pidof` confirmation, not
first-frame time. All 10 warm distributions have n=5 with the same PID. For
`ecosystem-core`, all 5 warm reps omitted `TotalTime`; the labelled `WaitTime`
fallback was used (157/95/86/78/88 ms) and start validity does not depend on it.

## Sustained load (3 × 30 s averages)

| App | Valid | Frames | Janky % | p50 (ms) | p95 (ms) | proc CPU % | ΔPSS (KB) | Slope (KB/s) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| material | 3/3 | 0 | — | — | — | 4.61 | -37.0 | 0.65 |
| primeng | 3/3 | 0 | — | — | — | 5.29 | -31.3 | 0.33 |
| ngzorro | 3/3 | 0 | — | — | — | 5.33 | -37.3 | 0.80 |
| taiga | 3/3 | 0 | — | — | — | 4.98 | -28.7 | 0.94 |
| spartan | 3/3 | 0 | — | — | — | 5.12 | -45.7 | 0.68 |
| ecosystem-grid | 3/3 | 539.0 | 48.40 | 44.0 | 71.7 | 31.02 | 1309.7 | 47.71 |
| ecosystem-charts | 3/3 | 25.7 | 96.29 | 36.0 | 63.7 | 7.07 | 130.7 | 3.03 |
| ecosystem-core | 3/3 | 0 | — | — | — | 3.87 | -134.7 | -0.80 |
| ecosystem-richtext | 3/3 | 56.3 | 35.30 | 21.7 | 36.3 | 5.29 | -408.3 | -7.76 |
| ecosystem-calendar | 3/3 | 0 | — | — | — | 3.15 | -118.7 | -1.11 |

Seven apps produced **zero new rendered frames** after the post-settle
`gfxinfo` reset despite 42–57 issued swipes. Their jank/percentiles are shown
as `—` because the harness nulls the synthetic 4950 ms values; these apps have
no sustained frame-pacing evidence from this run. This is an observation about
the interaction, not a performance ranking.

`ecosystem-grid` per-sample detail:

| Sample | Frames | Janky % | p50 (ms) | p95 (ms) | proc CPU % | ΔPSS (KB) | Slope (KB/s) |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 541 | 43.44 | 42 | 65 | 31.08 | 1275 | 45.84 |
| 2 | 656 | 43.90 | 42 | 69 | 36.12 | 1419 | 52.34 |
| 3 | 420 | 57.86 | 48 | 81 | 25.85 | 1235 | 44.95 |

## Honest scope

- These are **emulator SwiftShader** figures. They are not production GPU
  numbers and are not a NativeScript/Flutter comparison.
- `proc CPU %` is percent of **one core** (`nproc=4`); `top` values are
  instantaneous and were frequently `0.0`, so they are retained only for
  continuity with the earlier multi-sample run.
- Memory deltas over 30 s include GC and settle effects. Negative deltas are
  not evidence of a leak fix; the grid's small positive rise is monotonic in
  these three samples but is not proof of a leak.
- `gfxinfo` summary is cumulative after the post-settle reset; raw per-frame
  rows are ring-limited and were not retained.
- Physical OnePlus `b6640c8f` was not connected. Physical sustained CPU,
  warm-start and multi-sample results remain **pending**; no physical claim is
  made here.
