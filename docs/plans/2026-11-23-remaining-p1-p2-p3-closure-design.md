# Remaining P1/P2/P3 and Gates B/F closure — design

**Date:** 2026-11-23
**Status:** Validated with user; ready for implementation
**Follows:** `docs/plans/2026-11-23-non-ios-gap-closure-design.md` (Phases 0–3 complete)

## Problem

The first non-iOS closure pass closed all P0 items and made deferred features
explicit, but the issue register still contained open P1/P2/P3 items and Gates
B/F pending. A thorough inventory established three distinct classes:

- **Already stale (evidence exists, docs lag):** P1-01, P1-04, P1-05, P3-01,
  P3-02, P3-07. These need honest reconciliation, not code.
- **Correctly deferred/blocked:** P1-08 (no ES module loader; supported bundles
  are self-contained IIFEs), P2-08 (iOS runtime — no macOS/Xcode).
- **Genuinely open:** P1-02, P1-03, P1-09, P1-10, P1-11, P1-12; P2-01, P2-02,
  P2-03, P2-05, P2-06, P2-07, P2-09, P2-11, P2-12, P2-13; P3-04, P3-05, P3-06,
  P3-08.

## Decisions

- **Handle validation uses a real validated `u64` handle table with generation
  counters** (P1-02/P1-03). The C header, Kotlin JNI, Swift bindings and
  `scripts/check-ffi-declarations.py` are updated together. The emulator is
  re-verified after the ABI change; the physical OnePlus is offline, so the new
  ABI is explicitly `not verified` there rather than claimed.
- **Risky items use honest substitutes** (user-approved): P2-13 adds an in-repo
  second `JsEngine` implementation to prove the abstraction supports more than
  one engine — it is **not** Hermes/V8 and is labelled that way. P2-02 gets a
  feasibility spike; without real QuickJS bytecode support it is deferred with
  the attempted API recorded, not faked. P2-09 gets a GPU-free bridge → view-tree
  → renderer integration test; real window/GPU verification stays `not verified`.
- **No iOS/macOS runtime claim**, no WebView, and the event bridge wording stays
  “one delivery per event (canonical JS thread-local path + legacy FFI fallback)”.
- **Current repository, no worktree.** Every phase ends with the full gate
  (`cargo test --workspace`, ignored runtime suite, `pnpm test`, FFI check),
  commit/push and a clean tree.

## Phases

0. **Stale reconciliation.** Close P1-01, P1-04, P1-05, P3-01, P3-02, P3-07
   with the retained evidence; confirm P1-08 deferred and P2-08 blocked; define
   measurable Gate B/F criteria. Docs only.
1. **Errors and metrics** (P1-09, P1-11, P3-04). One workspace error enum with
   stable FFI codes; an Angular `ErrorHandler` provider forwarding to a host
   callback; `RuntimeMetrics` with dispatch/load counters and latency. New FFI
   and JNI accessors. Tests: Rust counters, an ignored AOT throw-to-ErrorHandler
   test, FFI declaration check.
2. **Lifecycle, handles and teardown** (P1-02, P1-03, P1-10, P2-03 → Gate B).
   Validated `u64` handle table with generation; `RuntimeController` integrated
   into `RuntimeBridge` with deterministic transition failures; JNI global-ref
   RAII, `initHost()` failure cleanup and `Activity.onDestroy()`. Tests: stale-
   handle sentinel, transition table, emulator instrumentation; physical
   re-verification recorded as pending.
3. **Native contract completeness** (P1-12, P2-01). Canonical `na-list`,
   `na-switch`, `na-activity-indicator` propagated end-to-end as tagged custom
   nodes and materialized in `NativeViewFactory`; declarative shim conformance
   fixture. Tests: contract, Rust mutation and Android factory assertions.
4. **Test infrastructure** (P2-05, P2-06, P2-07, P2-09). Ignored stress tests
   (10k dispatches, 1k create/destroy cycles); deterministic memory-growth
   analysis via handle/node registry baseline (RSS remains `not verified` because
   QuickJS `memory_usage()` returns `None`); one declarative conformance spec
   consumed by Rust and TS runners; GPU-free desktop integration test.
5. **Build and performance** (P2-02, P2-11). Bytecode feasibility spike with a
   truthful outcome; Rust size gate wired into CI using
   `rustCoreMaxBytesPerPlatform` with explicit skips.
6. **Engine abstraction** (P2-12, P2-13 → Gate F). `RuntimeBridge` generic over
   `JsEngine` or engine-neutral trait object; second in-repo reference engine
   proving multi-engine support. Hermes/V8 explicitly not integrated.
7. **Polish and final reconciliation** (P3-05, P3-06, P3-08). `Debug` derives,
   canonical `RuntimeBridge` naming (C handle typedef unchanged), SPDX MIT
   headers, all issue rows and gate statuses updated from retained evidence.

## Validation

Every claim must have a passing test or retained artifact; otherwise it is
written as `not verified`. Android claims must match current `adb` evidence.
No new WebView and no “single event path” claim are introduced anywhere.
