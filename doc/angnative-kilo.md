# Prompt for **Kilo Code CLI** — AngNative Phase 1 (TypeScript layer) + inspector

> Run from the repo root `angnative/`.
> Headless: `kilo run "$(cat doc/angnative-kilo.md)"`
> On your machine, ensure Node >= 22.12, pnpm 10.34.9, and (for the Rust side) cargo are installed.

---

You are implementing selected tasks of the **AngNative** Rust-first build plan
(`research/TaskList.txt`), Phases 1 and 5. AngNative renders Angular components as
real native views (no WebView) on phones, watches, TV and desktop from one Angular
codebase, with a **Rust host** as the only native backend.

## Canonical identity (do not violate)
- Product: **AngNative**; npm scope `@angnative/*`; element prefix `na-`;
  licence **MIT** (see `docs/identity.md`).
- Angular exactly `22.2.5`; TypeScript exactly `6.0.3`; Node `>= 22.12`;
  pnpm `10.34.9`.
- AOT only — no `@angular/compiler` in any runtime/test bundle; never import
  `@angular/core/testing` or `TestBed`.
- Zoneless only — zero `zone.js`.
- Angular private (`ɵ`) API allowed in exactly one file:
  `packages/platform/src/angular-internals.ts`.
- Unknown property writes must throw (spec §5.3).
- Every gate is mutation-tested (spec §10.7).

## Your assigned tasks

### T2 — `createElementRenderer` (one renderer, written once)
- Build `packages/core/src/renderer.ts` (or equivalent) exposing
  `createElementRenderer(registry, options)` — the Angular-facing renderer written
  ONCE, sitting above an element-factory SPI. No per-platform forks.
- Definition of done: used by the fake backend AND the Rust host; conformance
  suite green.

### T3 — The shared conformance suite
- `packages/testing/src/conformance.spec.ts` (or `packages/contract`) defining
  create / append / remove / prop / style semantics, view flattening, list
  recycling, capability degradation, error cases.
- Every backend passes it; it is the only place backend semantics are defined.

### T3a — The 7 mutation "teeth" specs
- For each conformance assertion, inject the original defect and confirm the
  suite fails with that defect's exact numbers. Commit the specs, not just docs.

### D3 — Native tree inspector
- A single self-contained HTML page: no CDN, no external script, no external
  stylesheet, no web font, no external URL of any kind. Reads the live host tree;
  does not re-render. Must show 0 external requests.

## Where to work
- TypeScript packages under `packages/*`. Create them if absent (`packages/core`,
  `packages/contract`, `packages/testing`, `packages/platform`).
- Do NOT touch `crates/*` (Rust side is owned by another track) or
  `packages/cli/*` (owned by OpenCode's track).
- Do NOT modify `Cargo.toml` or `package.json` root without noting it.

## Verification (run before committing)
1. `pnpm run check:toolchain`
2. `pnpm run build` (tsc -b exit 0)
3. `pnpm test` (all green)
4. mutation check: inject original defect, confirm gate fails with exact numbers

## Report
Write your report to `doc/angnative-kilo-report.md` with: what you implemented,
TDD evidence (RED/GREEN with commands + output), files changed, self-review,
concerns. Then report back under 15 lines: status, commit SHA + subject, one-line
test summary, concerns, report path.
