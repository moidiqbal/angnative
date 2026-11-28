# DOM-library parity — documented deferral (2026-11-27)

This is a **documentation and dependency-surface guard**, not a runtime test.
It does not claim DOM-heavy libraries were installed and failed. The native
renderer understands only `na-*` primitives mapped to native widgets; there is
no DOM/CSS/contenteditable shim. Concept demos for grid, charts, richtext and
calendar are native reimplementations (ADR 0003), not unchanged-library runs.

## Guard

`scripts/scope-guards.spec.ts` → test
`scope guards — documented deferrals, not runtime claims > keeps DOM-heavy libraries formally unsupported and undeclared`:

1. Asserts the canonical register row still contains
   `**Unsupported on native path**`.
2. Asserts no workspace `package.json` declares Material, PrimeNG, NG-ZORRO,
   Taiga, Spartan, AG Grid, ECharts, Tiptap/CKEditor, FullCalendar or GSAP.
3. Asserts the RxJS 7.8.2 row is still explicitly labelled "a spot-check, not
   general compatibility".
4. Asserts this evidence file exists.

## Not verified

Unchanged Material/PrimeNG/NG-ZORRO/Taiga/Spartan/AG Grid/ECharts/Tiptap/
FullCalendar/GSAP apps on the native path; no DOM shim; no end-to-end library
ingestion.
