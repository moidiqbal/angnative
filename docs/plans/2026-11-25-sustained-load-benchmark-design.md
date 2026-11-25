# Sustained-Load Android Benchmark — Design (2026-11-25)

Status: **Implemented on emulator (2026-11-25)** — `status=ok`, 30/30 valid samples; physical device remains pending. Evidence: `docs/evidence/ANDROID-SUSTAINED-LOAD.md`.

## Section 1 — Scope & data flow

- **Targets:** all 10 showcase APKs from `dist/device-testing/`, with an explicit package -> activity map. `ecosystem-core` uses `EcosystemCoreActivity`, not `MainActivity`.
- **Per app:** 5 cold starts + 5 warm starts, then **3 samples x 30 s** continuous alternating swipes. Approximately 15 min of scroll plus start/overhead.
- **Harness:** new `scripts/measure-sustained.ps1`, emulator-first via `-Device emulator-5554`; same script accepts a physical serial later.
- **Collection per sample:** foreground guard at start/end; `gfxinfo reset` after settle; `/proc/<pid>/stat` CPU deltas every 1 s (plus legacy `top` samples for continuity); `dumpsys meminfo` every 5 s; final `gfxinfo framestats` summary.
- **Output:** one raw JSON per device under `docs/benchmarks/sustained/`, with per-sample arrays, averages, medians, min/max/p95, device metadata, method, and explicit caveats. Historical JSON files remain untouched.
- **No app-code changes.** If the emulator is offline, the script attempts one boot using the existing emulator helper logic, then skips cleanly with a recorded reason.

## Section 2 — JSON schema & failure handling

**File:** `docs/benchmarks/sustained/<device>-<timestamp>.json`. Numeric fields are real numbers; unknown values are `null`, never `0`. Top level: `schema_version`, `measured_at`, `status` (`ok|partial|skipped`), `device` (model, API, serial, renderer, `wm size`, `CLK_TCK`, `nproc`), `method`, `apps[]`, `averages`, `caveats[]`, `errors[]`.

`apps[]` contains, per app: `name`, `package`, `activity`, `apk_path`, `starts.cold` and `starts.warm`. Each start group has `samples[]` (5), `mean/median/min/max/p95/n`, and both `process_ms` (pidof polling) and `am start -W TotalTime`. A warm rep is valid only if the PID stayed the same. Each sustained `samples[]` entry (3) has `sample`, `valid`, `duration_s`, `actual_swipes`, `swipe_geometry`, `foreground_start/end`, `frames`, `janky_pct`, `p50/p95/p99_ms`, `cpu` (`procstat_pct_of_one_core[]`, `top_pct[]`, avg/p95/max, `nproc`), `memory` (`[{t_s,pss_kb,webviews,views}]`, baseline/end/peak/delta/slope), and errors.

**Failure handling:** if the device is offline, try one emulator boot and wait up to 180 s; if still unavailable, write `status: skipped` with a reason and no metrics. Each app/sample has its own try/catch: start failure, PID exit, foreground mismatch, or parse failure produces `valid: false` with phase and stderr snippet, then continues to the next sample/app. Aggregates use only `valid: true` samples; with fewer than 2 valid samples, set `insufficient_samples: true` and publish no headline average.

**Foreground guard:** WAKEUP plus shade collapse, then both `topResumedActivity` and `mCurrentFocus` are checked. One retry on mismatch; otherwise the sample is invalid.

## Section 3 — Testing, evidence & docs

**Implementation order**

1. Write this plan with Sections 1-3 exactly as approved.
2. Create `scripts/measure-sustained.ps1` and the app map for all 10 packages/activities. Add `-SelfTest`: synthetic fixtures for `am start -W`, `pidof`, `/proc/<pid>/stat`, `top`, `dumpsys meminfo`, and `gfxinfo framestats` verify parsing/aggregation without a device. No app-code changes.
3. Static checks: PowerShell parse check, `-SelfTest`, `git diff --check`. Existing `pnpm test` and `cargo test --workspace` are rerun at the end because docs/scripts should not touch runtime, but baseline must remain green.

**Evidence run**

- Capture `adb devices -l` before/after boot. Boot `AngNative34` via `scripts/run-android-emulator.ps1` if needed.
- Install APKs from `dist/device-testing/` if missing, then run one emulator command with `-Samples 3 -DurationSeconds 30 -StartReps 5`.
- Save raw `docs/benchmarks/sustained/emulator-<timestamp>.json`; write `docs/evidence/ANDROID-SUSTAINED-LOAD.md` with exact command, device metadata, valid/invalid counts, per-app table, and caveats.

**Docs honesty gates**

- Update `docs/BENCHMARKS.md`, `docs/SHOWCASE-TRACKER.md`, `docs/UNSUPPORTED-FEATURES.md`, `docs/architecture/issue-register.md` Gate G, and `tasklist.txt` only if JSON has `status: ok/partial` with retained samples.
- `README.md` / `docs/COMPARISON.md` get no new headline numbers unless valid evidence exists; no NativeScript/Flutter comparison.
- Physical OnePlus stays `pending`; if online later, rerun the same script with its serial and keep a separate file/table.
