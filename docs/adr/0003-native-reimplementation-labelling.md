# ADR 0003 — Label native equivalents explicitly, never claim unchanged compatibility

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Angular Material, PrimeNG, NG-ZORRO, Taiga UI, Spartan UI, AG Grid, ECharts, Tiptap, FullCalendar and GSAP are DOM/CSS/JS libraries. The native renderer supports only `na-*` primitives mapped to `TextView`/`UIKit`/native widgets. Claiming these libraries "work natively" would be false.
- **Decision:** Compatibility reports use the fixed status vocabulary from the mission brief. A native equivalent is always labelled `Native reimplementation required`, with the exact library version listed as "design reference only — not bundled". The 11-category matrix (`docs/ECOSYSTEM-COMPATIBILITY.md`) records `Not yet tested` where no test ran.
- **Consequences:**
  - Community sees an honest compatibility story instead of marketing.
  - Some categories remain unimplemented; they are listed in `docs/UNSUPPORTED-FEATURES.md` rather than hidden.

## 2026-11-23 addendum — flagship-only Angular shell scope

- **Decision:** `showcases/ecosystem-core` is the single supported Angular-driven
  shell. It keeps the unified runtime bridge alive, loads a real AOT bundle, and
  has a verified native tap into a live listener on emulator and OnePlus
  (`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`). Its remaining concept
  sections and the other 9 showcases stay imperative native reimplementations by
  design; `platforms/android` keeps the legacy host.
- **Consequence:** P0-08 is closed as a scope decision, not by rewriting every
  shell. Gate D is passed for the flagship E2E with this limitation stated. No
  claim of Angular parity is made for the other shells.
