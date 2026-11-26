# Android emulator multi-sample benchmark evidence

**Date:** 2026-11-23
**Phase:** Phase 2 of `docs/plans/2026-11-23-non-ios-gap-closure-design.md`

## Device and command

`adb devices -l` at measurement time contained only:

```text
emulator-5554  device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1
```

Properties: `sdk_gphone64_x86_64`, Android 14, `x86_64`. This is the SwiftShader
software-rendered emulator, not a physical GPU.

```powershell
pwsh -File scripts/measure-multisample.ps1 -SampleCount 3
```

Raw retained output: `docs/benchmarks/emulator-multisample.json`.

## Method

For each of the four apps in the earlier single-burst frame-pacing run:

1. force-stop, reset `gfxinfo`, launch `com.angnative.app.MainActivity`, settle 3 s;
2. perform a 6-swipe interaction burst;
3. sample `top -n 1 -p <pid>` between swipes for instantaneous CPU;
4. read `dumpsys gfxinfo <pkg> framestats`;
5. repeat 3 times and average the samples.

## Results (3-sample averages)

| App | Frames | Janky % | p50 ms | p95 ms | CPU avg % | CPU max % |
|---|---:|---:|---:|---:|---:|---:|
| ecosystem-grid | 127.3 | 48.96 | 42.7 | 93.0 | 6.15 | 53.8 |
| ecosystem-charts | 7.0 | 100.00 | 32.7 | 1150.0 | 0.00 | 0.00 |
| ecosystem-richtext | 1.0 | 100.00 | 1183.3 | 1183.3 | 0.00 | 0.00 |
| ecosystem-calendar | 8.0 | 83.20 | 87.7 | 1066.7 | 0.21 | 3.8 |

## Honest scope

- These are **emulator SwiftShader** figures. They are not production GPU
  numbers and are not a NativeScript/Flutter comparison.
- Very low frame counts for charts, richtext and calendar mean their jank and
  percentile figures are dominated by the short burst and software rasterization;
  do not treat them as stable app rankings.
- CPU values are instantaneous `top` samples taken between swipes and are frequently
  `0.0`; they are **not** sustained CPU utilization.
- At the time of this multisample document, `b6640c8f` was not connected and
  only the emulator was listed; the earlier single-burst physical grid sample in
  `docs/benchmarks/physical/results.json` was not a multi-sample result. That
  scope note is historical: physical multi-sample frame pacing, CPU and
  warm-start were later measured on `b6640c8f` and retained in
  `docs/benchmarks/sustained/b6640c8f-20261126-170816.json` (`status=ok`,
  33/33 valid). See `docs/evidence/ANDROID-SUSTAINED-LOAD.md`.
- No WebView was introduced; the benchmark apps are the existing native showcases.
