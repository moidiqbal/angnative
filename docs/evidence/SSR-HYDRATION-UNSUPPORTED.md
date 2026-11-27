# SSR / hydration — documented deferral (2026-11-27)

This is a **documentation and dependency-surface guard**, not a runtime test.
It does not claim SSR was tried and failed. AngNative renders Angular AOT output
directly into a native `na-*` widget tree; there is no server render stage, no
HTML document and no DOM to hydrate. `docs/REQUIREMENTS.md` excludes replacing
Angular Web/SSR.

## Guard

`scripts/scope-guards.spec.ts` → test
`scope guards — documented deferrals, not runtime claims > keeps SSR/hydration formally unsupported and undeclared`:

1. Asserts the canonical register row in `docs/UNSUPPORTED-FEATURES.md` is still
   `| SSR / hydration | **Unsupported (out of scope)** |`.
2. Asserts no workspace `package.json` declares `@angular/platform-server`.
3. Asserts this evidence file exists.

Exact command and result are recorded in `docs/evidence/HMR-ZEROCOPY-BINARY-PROTOCOL.md`
's successor gate table once `pnpm test` is run against the final tree.

## Not verified

No SSR build, no hydration run, no server/client timing, no iOS claim.
