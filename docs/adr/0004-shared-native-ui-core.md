# ADR 0004 — Extract shared native UI code into @angnative/native-ui-core

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Eight showcase projects each carried a copy of `NativeViewFactory.kt` and ~109 MB of Rust static libraries (~870 MB duplicated). Two factory "variants" differed only by a trailing newline. Duplication would make fixes drift.
- **Decision:** Add `packages/native-ui-core/` as a real Android library with all 8 PlatformOp callbacks, a pluggable `viewCreator`, `NativeUiTheme`, `StyleCodec` and a typed `PlatformOp` decoder. Existing showcases remain untouched for now (backwards compatibility); new showcases consume the library.
- **Consequences:**
  - Positive: single canonical implementation, 6 JUnit tests, AAR produced and build verified.
  - Negative: existing showcases still duplicate the factory until a migration pass; Rust libs intentionally stay per-app (they belong to `angnative-host-ffi`, not this package).
