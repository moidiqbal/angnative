# ADR 0003 — Label native equivalents explicitly, never claim unchanged compatibility

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Angular Material, PrimeNG, NG-ZORRO, Taiga UI, Spartan UI, AG Grid, ECharts, Tiptap, FullCalendar and GSAP are DOM/CSS/JS libraries. The native renderer supports only `na-*` primitives mapped to `TextView`/`UIKit`/native widgets. Claiming these libraries "work natively" would be false.
- **Decision:** Compatibility reports use the fixed status vocabulary from the mission brief. A native equivalent is always labelled `Native reimplementation required`, with the exact library version listed as "design reference only — not bundled". The 11-category matrix (`docs/ECOSYSTEM-COMPATIBILITY.md`) records `Not yet tested` where no test ran.
- **Consequences:**
  - Community sees an honest compatibility story instead of marketing.
  - Some categories remain unimplemented; they are listed in `docs/UNSUPPORTED-FEATURES.md` rather than hidden.
