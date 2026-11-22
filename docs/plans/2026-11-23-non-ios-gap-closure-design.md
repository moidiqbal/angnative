# Non-iOS/Mac gap closure — design

**Date:** 2026-11-23
**Status:** Validated with user; ready for implementation

## Problem

After the Android UI-thread marshalling and live-tap work (`28b535b`), the
remaining open items were all outside iOS/macOS. Several were genuinely open;
others were already fixed but still described as outstanding in canonical docs.
Mixing the two makes the repository's honesty claims unreliable.

## Decisions

- **P0-08 is closed as a flagship-only scope decision.** `showcases/ecosystem-core`
  is the supported Angular shell: it keeps the unified runtime bridge alive, loads
  a real AOT bundle, and has a verified native tap into a live listener. Its
  remaining concept sections and the other 9 showcases stay deliberately
  imperative native reimplementations, consistent with ADR 0003. This is a scope
  decision, not a claim of Angular parity.
- **Gate D is passed for that flagship E2E** with an explicit limitation; it is
  not a claim that every shell is Angular-driven.
- **Benchmarks stay honest:** emulator multi-sample/CPU can be collected now;
  physical-device multi-sample remains unverified while the OnePlus is offline.
- **Deferred features get explicit status, not silent removal.** DOM-heavy
  libraries, real-time animation easing, zero-copy/binary transport, web AOT
  migration, HMR and SSR remain unsupported/deferred with rationale.
- **No new WebView, no single-path claim.** The event bridge remains “one
  delivery per event (canonical JS thread-local path + legacy FFI fallback)”.
- **Current repository, no worktree.** No iOS/macOS verification claim.

## Phases

1. **Docs/evidence consistency.** Fix stale lines in `ROADMAP.md`,
   `VERIFICATION-REPORT.md`, `docs/architecture/issue-register.md`,
   `docs/architecture/ARCHITECTURE-VERIFIED-2026-11-22.md`,
   `docs/UNSUPPORTED-FEATURES.md`, and related notes. Keep the OnePlus
   screenshot-not-retained caveat.
2. **Scope formalization.** Record the flagship-only decision and update P0-08 /
   Gate D / ADR cross-references. No shell code rewrite.
3. **Benchmarks.** Collect emulator multi-sample frame pacing and CPU using
   existing scripts; retain raw JSON. Mark physical multi-sample as not verified
   because `b6640c8f` is not connected.
4. **Feature status.** Convert remaining open rows into explicit
   unsupported/deferred decisions with reasons. No new implementation in this
   phase.

## Validation

After each phase run the full gate: `cargo test --workspace`,
`cargo test -p angnative-runtime-bridge -- --ignored`, `pnpm test`, and
`python scripts/check-ffi-declarations.py`; then commit and push. Android claims
must match current `adb` evidence. A claim with no passing test or retained
artifact is written as “not verified”.
