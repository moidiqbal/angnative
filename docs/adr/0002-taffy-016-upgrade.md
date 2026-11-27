# ADR 0002: Upgrade the isolated layout adapter to Taffy 0.16.0

- Status: Accepted
- Date: 2026-11-27
- Supersedes only the pinned version in ADR 0001; ADR 0001 remains the record
  of why Taffy is isolated behind `taffy_adapter.rs`.

## Context

ADR 0001 pinned Taffy 0.15.1 and deliberately confined its types, tree
operations and measurement callback to `angnative-host-layout/src/taffy_adapter.rs`.
On 2026-11-27 the workspace manifest and lockfile were upgraded to Taffy 0.16.0.
The boundary is intended to make such upgrades local.

## Decision

- Keep Taffy as the layout engine and keep the `taffy_adapter.rs` boundary.
- Update the workspace pin, lockfile and version-labelling comments/docs to
  0.16.0 rather than renaming or rewriting ADR 0001.
- No AngNative-specific CSS layout engine is introduced.

## Verification recorded 2026-11-27

- `cargo test --workspace` → 171 passed / 0 failed / 17 ignored, including
  `text_and_image_measurement_callbacks_are_used` in `angnative-host-layout`.
- `python scripts/check-ffi-declarations.py` → `FFI declaration check OK: 40 JNI
  methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 31
  header declarations; iOS unified staticlib wiring OK`.
- Lockfile was regenerated minimally with `cargo update -p taffy --precise 0.16.0`
  (3 insertions / 2 deletions) rather than a broad refresh.

## Consequences

- The Android native rebuild and APK size cap remain the deployment gates; the
  rebuild must be repeated before any new size claim.
- Platform shells keep consuming layout output through the same C ABI; no JNI
  signature changes are implied by this ADR.
- iOS/macOS execution remains unverified on this Windows host.
