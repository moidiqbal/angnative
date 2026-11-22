# Desktop GPU-free bridge / view-tree integration

**Date:** 2026-11-23
**Scope:** GPU-free integration only.

## What was added

`crates/angnative-desktop/tests/gpu_free_bridge_view_tree_integration.rs`
exercises the desktop bridge/view-tree integration without a GPU: `DesktopBridge`
drives native view-tree operations and the resulting `LocalViewTree` is asserted
in-process.

## Scope caveats

- Real window creation, GPU context and rendered output are **not verified**.
  This test is not a desktop rendering or visual-fidelity proof.
- No device or platform ABI re-verification is claimed by this test.
- No iOS/macOS runtime claim is made.

**Related register row:** P2-09.
