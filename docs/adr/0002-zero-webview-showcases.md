# ADR 0002 — Zero WebView for all native-mode showcases

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Community reviewers asked for compatibility demonstrations of Angular UI libraries (Angular Material, PrimeNG, NG-ZORRO, Taiga UI, Spartan UI). The mission brief made "Zero WebView for all native-mode demonstrations" non-negotiable. An earlier Capacitor/WebView approach was started and then discarded.
- **Decision:** Every native showcase APK is a standalone Android project that drives native widgets from Rust `PlatformOp`s through the JNI bridge. DOM/CSS libraries are never bundled in an APK; they are used only as design-language references, and each report labels the showcase a "native reimplementation".
- **Consequences:**
  - Positive: performance claims are not undermined by an embedded browser; reviewer concern about cross-platform frame loss is addressed honestly.
  - Negative: showcases are not "unchanged third-party compatibility" — this is stated explicitly in every compatibility report.
  - Verification: `dumpsys meminfo <pkg>` reports `WebViews: 0` for all eight APKs; source scans show no `android.webkit` usage.
