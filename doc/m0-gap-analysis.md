# M0 gap analysis — can Angular render to native views?

**Date:** 2026-11-11 · **Status:** resolved — the SPI path works.

## Question

Can an Angular 22 application render into a non-DOM tree through Angular's
public extension points, so that a native backend can take over?

## Answer

**Yes.** Angular 22.2.5 was bootstrapped in Node with no DOM present, using a
custom `Renderer2`, and rendered a real AOT-compiled component tree. The
required extension points are `Renderer2` / `RendererFactory2` (public API) plus
seven `ɵ` internals — `ɵinternalCreateApplication`, `ɵsetDocument`,
`ɵINJECTOR_SCOPE`, `ɵIMAGE_CONFIG`, `ɵɵdefineComponent`, `ɵɵelementStart` and
`ɵɵelementEnd` — all isolated in `packages/platform/src/angular-internals.ts`,
whose header documents every import and why it is needed.

This mirrors the approach NativeScript's Angular integration has used in
production for years, so the risk is not "does it work" but "how far does the
contract reach" — answered in the table below.

## Measured renderer contract (Angular 22.2.5)

| # | Behaviour Angular relies on | Required renderer support |
|---|---|---|
| F4 | `createRenderer(null, cmpDef)` | Tolerate a null host; the type argument is the component definition — a null type must be tolerated as well |
| F5 | `removeChild(null, child, isHostElement, requireSync)` | The parent argument is null — resolve the parent from the node itself |
| F6 | `element.textContent = ''` | Clearing a container must remove its children |
| F7 | Classes are written with `setAttribute('class', value)` | Store the class list as an attribute, derive the list from it |
| F8 | Text is written with `setValue(node, value)` onto `nodeValue` | Implement the DOM `nodeValue` property on text nodes |
| F9 | Angular reads `parentNode` and `nextSibling`; calls `renderer.destroy()` | Track the parent, compute the next sibling, implement `destroy()` |
| F10 | A property write for an input Angular does not know (`setProperty`) | **Throw** — a native host cannot apply it, and a silent no-op renders an empty value |

Every row has a test in `packages/testing/src/fake-renderer.spec.ts` (and F10
additionally in `packages/testing/src/fake-backend.spec.ts`, through the
factory path a real backend uses).

## The silent failure that shaped the design

*Provenance:* the JIT half of the claim below was observed during the spike
that preceded this plan, with a JIT-compiled component. It cannot be
re-measured in this repository, because JIT needs `@angular/compiler`, which
is banned here. The AOT half is proved by the fixture pipeline in
`packages/testing/fixtures`.

**Signal inputs are AOT-only.** In JIT, `[label]="expr"` on a component whose
input is declared with `input()` is *not* recognised as a directive input.
Angular falls through to `renderer.setProperty(hostElement, 'label', value)`.
On a web page that writes a DOM property; on a native host it writes nothing —
and the component renders an empty value with **no error anywhere**.

AOT emits `inputs: { label: [1, "label"] }` (`InputFlags.SignalBased`), which is
why the fixture pipeline compiles with `ngc` and why the test harness asserts
that no unknown property write ever reaches the renderer.

**Design rule adopted:** a native renderer must treat an unknown property write
as an error, not a no-op. `@nativeangular/testing` enforces this, so the
failure mode cannot reach a device.

## Platform recipe

Platform providers: `PLATFORM_ID`, `DOCUMENT`, `RendererFactory2`, `CSP_NONCE`
(`null`), `IMAGE_CONFIG` (warnings disabled).
Application providers: `ɵINJECTOR_SCOPE` (`'root'`), `ErrorHandler`, `APP_ID`.

`ɵINJECTOR_SCOPE` and `ErrorHandler` **must** be application providers. In the
platform injector the internal error handler resolves circularly — measured.

## Remaining risks carried into M1

1. **Styles.** Angular's `RendererType2` carries component styles; a native
   backend must map them to native properties. The style subset is defined in
   the design spec and is M1 work.
2. **View flattening.** A component host with no style, property or listener
   should not create a native view. The contract suite in M1 must lock the
   behaviour so both backends agree.
3. **Projection and slots.** `<ng-content>` maps onto the `slot` element kind.
   Not yet exercised by the fixture.
4. **Animations.** Angular's animation engine drives DOM elements, and there is
   no DOM here; M1 ships without `@angular/animations` and drives native
   animations directly.
5. **The element registry is declared, not wired.** `ELEMENT_KINDS` and
   `ElementRegistry` exist in `@nativeangular/core`, but nothing constructs a
   registry from a backend: `NativeBackend.elements` is empty and neither
   `elements` nor `capabilities` has a reader yet. The resolver that maps a
   backend's element factories onto the kind vocabulary ships in M2.

## Version floors

- Angular `22.2.x`, TypeScript `6.0.x` — pinned to `22.2.5` and `6.0.3`.
  `@angular/compiler-cli`'s TypeScript peer range is `>=6.0 <6.1`, but it is an
  optional peer, so an incompatible TypeScript is not rejected by the installer;
  `pnpm run check:toolchain` fails on a mismatch.
- Node `>=22.12` (hard floor, enforced by `pnpm run check:toolchain`),
  `22.22.3` recommended — `@angular/compiler-cli@22.2.5` declares
  `engines.node: ^22.22.3 || ^24.15.0 || >=26.0.0`, so Node 23 and Node 25 are
  outside the supported lines. CI pins `22.22.3`.
