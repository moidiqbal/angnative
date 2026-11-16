# NativeAngular M0 — Spike & Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove out the NativeAngular core: an Angular 22 app compiled AOT renders through a custom `Renderer2` into a DOM-less node tree in Node — the foundation every backend (Fabric, Rust) will sit on.

**Architecture:** Three packages. `@nativeangular/core` holds the backend-agnostic SPI (element kinds, element factories, backend contract, capability declarations). `@nativeangular/platform` provides the non-DOM Angular platform — the small set of providers, the document shim and the bootstrap call that let Angular run without `@angular/platform-browser`. `@nativeangular/testing` provides a fake backend whose renderer implements the *measured* Angular 22 renderer contract, so components can be rendered and asserted in Node with no simulator.

**Tech Stack:** Angular 22.2.5 (AOT only, zoneless), TypeScript 6.0.3, pnpm 10 workspaces, Vitest 3, Node ≥ 22.12 (CI pins 22.22.x), no zone.js, no `@angular/platform-browser` in runtime code.

## Global Constraints

- **Angular version is exactly `22.2.x`** (`22.2.5` at time of writing). Do not float to `22.3.0-next.*`.
- **TypeScript is exactly `6.0.x`** (`6.0.3`). `@angular/compiler-cli@22.2.5` declares `peerOptional typescript@">=6.0 <6.1"` — but the peer range is *optional*, so the installer does **not** reject an incompatible TypeScript. The repository pins `6.0.3` and `pnpm run check:toolchain` fails on a mismatch. Do not run `pnpm update` on typescript.
- **Node floor `>=22.12`**, hard error below that. Angular 22's own engine floor is `^22.22.3 || ^24.15.0 || >=26.0.0`; the toolchain check **warns** below `22.22.3` but does not fail (the dev sandbox runs 22.19.0). CI pins Node `22.22.x`.
- **AOT only.** No `@angular/compiler` in any runtime or test bundle. `@angular/compiler-cli` is a devDependency only.
- **Zoneless only.** No `zone.js` anywhere. Angular 22.2.5's `ɵinternalCreateApplication` always prepends `provideZonelessChangeDetectionInternal()` to the application providers, so the platform is zoneless by construction — no `provideZonelessChangeDetection()` call is needed.
- **`@angular/platform-browser` is banned from runtime code.** It pulls in `DomAdapter`, `DomRendererFactory2` and the HTML sanitizer, all of which assume a DOM.
- **Angular private (`ɵ`) API is allowed in exactly one file:** `packages/platform/src/angular-internals.ts`. Every `ɵ` import used anywhere else is a review failure. The file must keep a table of what it imports and why.
- **npm scope is `@nativeangular/*`.** Never use `ng-native`, `@ng-native/*` or `@angular/*` — those belong to other projects.
- **One live application per realm.** Angular allows a single platform injector per process, so a second `bootstrapNativeAngular` releases the previous platform (and with it the previous application). The test harness must account for this; a teardown helper is a Task 5 deliverable.
- Every package's README carries the trademark notice: *"NativeAngular is an independent community project, not affiliated with or endorsed by Google LLC, the Angular team, Expo or Meta. 'Angular' is a trademark of Google LLC and is used here only to describe compatibility."*
- Licence is MIT. Third-party code must be attributed in `THIRD_PARTY_NOTICES.md`.
- Every task ends with a `git commit`. Every task is independently testable: `pnpm test` must be green at the end of each task.

## Verified spike results (read this before starting)

The recipe below is **not hypothetical** — it was executed end-to-end against Angular 22.2.5 in Node before this plan was written. The measured findings are load-bearing:

| # | Finding | Consequence for this plan |
|---|---|---|
| F1 | Angular 22 can bootstrap with a custom `Renderer2` and **no DOM**, using `createPlatformFactory(platformCore, …)` + `ɵinternalCreateApplication({ rootComponent, appProviders, platformRef })` (with `platformProviders: []` — the platform injector already carries them) + `ɵsetDocument(shim)` | Task 3 implements exactly this |
| F2 | Required providers: platform → `PLATFORM_ID`, `DOCUMENT`, `RendererFactory2`, `CSP_NONCE: null`, `IMAGE_CONFIG` with warnings disabled; app → `ɵINJECTOR_SCOPE: 'root'`, `ErrorHandler`, `APP_ID` | Task 3, verbatim |
| F3 | `ɵINJECTOR_SCOPE: 'root'` and `ErrorHandler` must live in **app** providers, not platform providers — in platform providers the internal error handler resolves circularly | Task 3 puts them in `APP_PROVIDERS` |
| F4 | Angular calls `renderer.createRenderer(null, cmpDef)` — the **host** is null and the type argument is the component definition; a null type must be tolerated as well | Task 4 handles it |
| F5 | Angular calls `renderer.removeChild(null, child, isHostElement, requireSync)` — the parent argument is **null**; the renderer must resolve the parent itself | Task 4 implements parent tracking |
| F6 | Angular clears a container with `element.textContent = ''` | Task 4 implements the `textContent` setter |
| F7 | Classes are written with `setAttribute('class', value)`, **not** `addClass` | Task 4 stores classes as an attribute; the class list is derived from it |
| F8 | Text is written with `setValue(textNode, value)` onto the DOM `nodeValue` property | Task 4 writes `nodeValue` |
| F9 | Angular reads `parentNode` and `nextSibling`, and calls `renderer.destroy()` on view teardown | Task 4 implements all three |
| F10 | **Signal inputs are AOT-only.** In JIT, `[label]="…"` silently falls through to `renderer.setProperty(host, 'label', value)` — a no-op on a native host. AOT emits `inputs: { label: [1, "label"] }` (`InputFlags.SignalBased`) | Task 5 compiles fixtures with `ngc`; Task 5 also adds a **negative test** that the renderer throws on unknown property writes, so this silent failure can never ship again |
| F11 | `ngc` refuses `moduleResolution: "node10"` under TypeScript 6 (TS5107) | Task 5 fixture tsconfig uses `"module": "preserve", "moduleResolution": "bundler"` |

---

### Task 1: Repository skeleton and toolchain verification

**Files:**
- Create: `package.json`, `pnpm-workspace.yaml`, `.npmrc`, `.gitignore`, `.editorconfig`, `tsconfig.base.json`, `tsconfig.json`, `vitest.config.ts`
- Create: `scripts/check-toolchain.mjs`, `scripts/check-toolchain.spec.ts`
- Create: `LICENSE`, `README.md`, `THIRD_PARTY_NOTICES.md`

**Interfaces:**
- Consumes: nothing (first task).
- Produces:
  - `pnpm test` → runs every `*.spec.ts` in every workspace package.
  - `pnpm run check:toolchain` → prints a toolchain table and exits 0 (or 1 on a hard error).
  - `pnpm run build` → compiles every package with `tsc -b` (no bundler yet; M0 ships plain ESM).
  - `tsconfig.base.json` → the shared compiler options every package extends.

- [ ] **Step 1: Write the failing toolchain spec**

Create `scripts/check-toolchain.spec.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { checkToolchain } from './check-toolchain.mjs';

describe('toolchain', () => {
  it('accepts the supported Node versions', () => {
    const result = checkToolchain(process.versions.node);
    expect(result.errors).toEqual([]);
  });

  it('rejects a Node version below the hard floor', () => {
    const result = checkToolchain('v20.0.0');
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toContain('22.12');
  });

  it('warns below the Angular 22 engine floor but does not fail', () => {
    const result = checkToolchain('v22.19.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings.some((w: string) => w.includes('22.22.3'))).toBe(true);
  });
});

describe('toolchain — Angular 22 engine range', () => {
  it('does not warn on the 22.x line at the engine floor', () => {
    const result = checkToolchain('v22.22.3');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toEqual([]);
  });

  it('does not warn on a supported 24.x version', () => {
    const result = checkToolchain('v24.15.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toEqual([]);
  });

  it('does not warn on a supported 26.x version', () => {
    const result = checkToolchain('v26.0.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toEqual([]);
  });

  it('warns but does not fail on the unsupported 23.x line', () => {
    const result = checkToolchain('v23.5.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toHaveLength(1);
    expect(result.warnings[0]).toContain('22.22.3');
  });

  it('warns but does not fail on the unsupported 25.x line', () => {
    const result = checkToolchain('v25.0.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toHaveLength(1);
  });

  it('warns but does not fail on a 24.x version below 24.15.0', () => {
    const result = checkToolchain('v24.14.0');
    expect(result.errors).toEqual([]);
    expect(result.warnings).toHaveLength(1);
  });
});
```

- [ ] **Step 2: Create the toolchain checker and run the test to verify it fails**

Create `scripts/check-toolchain.mjs`:

```js
export const HARD_MIN_NODE = '22.12';
export const ANGULAR_ENGINE_NODE = '22.22.3';

/**
 * `@angular/compiler-cli@22.2.5` declares `engines.node` as
 * `^22.22.3 || ^24.15.0 || >=26.0.0`: Node 23 and Node 25 are not supported
 * lines, and Node 24 is only supported from 24.15.0. Copied verbatim so a
 * future Angular bump is a one-line edit here.
 */
export const ANGULAR_ENGINE_RANGE = `^${ANGULAR_ENGINE_NODE} || ^24.15.0 || >=26.0.0`;

function parse(v) {
  // The patch component is optional: the hard floor is declared as "22.12".
  const m = /^v?(\d+)\.(\d+)(?:\.(\d+))?/.exec(v);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3] ?? 0)] : null;
}

function cmp(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

/**
 * Minimal evaluator for the two comparators Angular's engine range uses:
 * `^` (at or above the given version, without changing the left-most non-zero
 * component) and `>=` (at or above, unbounded). Hand-rolled on purpose: this
 * repo keeps its dependency set minimal and pinned, so `semver` is not a
 * direct dependency.
 */
function parseRange(range) {
  return range.split('||').map((clause) => {
    const m = /^\s*(>=|\^)\s*(\d+)\.(\d+)\.(\d+)\s*$/.exec(clause);
    if (!m) throw new Error(`Unsupported engine range clause: "${clause}"`);
    const min = [Number(m[2]), Number(m[3]), Number(m[4])];
    let max = null; // null means unbounded
    if (m[1] === '^') {
      if (min[0] > 0) max = [min[0] + 1, 0, 0];
      else if (min[1] > 0) max = [0, min[1] + 1, 0];
      else max = [0, 0, min[2] + 1];
    }
    return { min, max };
  });
}

const ENGINE_RANGES = parseRange(ANGULAR_ENGINE_RANGE);

/** True when the version falls inside one of Angular 22's supported Node ranges. */
function satisfiesEngineRange(v) {
  return ENGINE_RANGES.some(
    ({ min, max }) => cmp(v, min) >= 0 && (max === null || cmp(v, max) < 0),
  );
}

export function checkToolchain(nodeVersion) {
  const errors = [];
  const warnings = [];
  const v = parse(nodeVersion);
  if (!v) {
    errors.push(`Unrecognised Node version "${nodeVersion}".`);
    return { errors, warnings };
  }
  const floor = parse(HARD_MIN_NODE);
  if (cmp(v, floor) < 0) {
    errors.push(`Node ${nodeVersion} is below the hard floor ${HARD_MIN_NODE}.`);
  } else if (!satisfiesEngineRange(v)) {
    warnings.push(
      `Node ${nodeVersion} is outside the Angular 22 engine range ${ANGULAR_ENGINE_RANGE}; ` +
        'the toolchain still works but npm prints an EBADENGINE warning.',
    );
  }
  return { errors, warnings };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const { errors, warnings } = checkToolchain(process.versions.node);
  for (const w of warnings) console.warn(`warn: ${w}`);
  for (const e of errors) console.error(`error: ${e}`);
  console.log(`node ${process.versions.node} · npm ${process.env.npm_config_user_agent ?? 'n/a'}`);
  process.exit(errors.length ? 1 : 0);
}
```

Run: `pnpm vitest run scripts/check-toolchain.spec.ts`
Expected: FAIL — `Cannot find module './check-toolchain.mjs'` (it does not exist yet).

- [ ] **Step 3: Write the repository skeleton**

`package.json`:

```json
{
  "name": "nativeangular",
  "version": "0.0.0",
  "private": true,
  "description": "One Angular codebase. Every native platform.",
  "license": "MIT",
  "type": "module",
  "engines": { "node": ">=22.12" },
  "packageManager": "pnpm@10.34.9",
  "scripts": {
    "check:toolchain": "node scripts/check-toolchain.mjs",
    "test": "vitest run",
    "test:watch": "vitest",
    "build": "tsc -b",
    "compile:fixtures": "node scripts/compile-fixtures.mjs"
  },
  "devDependencies": {
    "@angular/compiler-cli": "22.2.5",
    "@types/node": "22.19.0",
    "typescript": "6.0.3",
    "vitest": "3.2.4"
  }
}
```

`pnpm-workspace.yaml`:

```yaml
packages:
  - 'packages/*'
```

`.npmrc`:

```
engine-strict=false
strict-peer-dependencies=false
```

`.gitignore`:

```
node_modules/
dist/
out/
*.tsbuildinfo
.DS_Store
```

`.editorconfig`:

```
root = true

[*]
charset = utf-8
indent_style = space
indent_size = 2
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
```

`tsconfig.base.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM"],
    "experimentalDecorators": true,
    "useDefineForClassFields": false,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "composite": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true
  }
}
```

`tsconfig.json` — the solution file. Task 2 adds the first project reference; later tasks add theirs.

```json
{
  "files": [],
  "references": []
}
```

`vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['packages/*/src/**/*.spec.ts', 'scripts/**/*.spec.ts'],
    environment: 'node',
  },
});
```

- [ ] **Step 4: Register the first project reference**

When Task 2 creates `packages/core`, add it to the root solution file:

```json
{
  "files": [],
  "references": [{ "path": "./packages/core" }]
}
```

- [ ] **Step 5: Add the licence, README and third-party notices**

`LICENSE` — MIT, Copyright (c) 2026 NativeAngular contributors.

`README.md`:

```markdown
# NativeAngular

**One Angular codebase. Every native platform.**

NativeAngular renders Angular components as real native views — no WebView — on
phones, watches, TV and desktop, from one codebase.

> **Status: pre-alpha (M0).** The core renderer platform is being proven in Node.
> There is no device build yet. See `docs/architecture/m0-gap-analysis.md`.

NativeAngular is an independent community project, not affiliated with or endorsed
by Google LLC, the Angular team, Expo or Meta. "Angular" is a trademark of Google
LLC and is used here only to describe compatibility.
```

`THIRD_PARTY_NOTICES.md`:

```markdown
# Third-party notices

NativeAngular is MIT-licensed. It builds on, and acknowledges:

- **Angular** — MIT, © Google LLC. Used as the component model, compiler and DI.
- **React Native (Fabric)** — MIT, © Meta Platforms, Inc. Used as the v0.1 phone backend.
- **NativeScript** — MIT, © Progress Software. Its Angular integration is the
  reference implementation for a DOM-less Angular platform; the platform recipe in
  `packages/platform` follows that proven approach.

No third-party code is vendored at M0.
```

- [ ] **Step 6: Install dependencies and run the tests**

Run: `corepack enable pnpm && pnpm install`
Expected: installs `@angular/compiler-cli@22.2.5`, `typescript@6.0.3`, `vitest`. If npm reports `EBADENGINE` for Node, that is the known Angular 22 engine warning — it is a warning, not an error.

Run: `pnpm vitest run scripts/check-toolchain.spec.ts`
Expected: 3 tests PASS.

Run: `pnpm run check:toolchain`
Expected: prints `node v22.19.0 …` plus the EBADENGINE-style warning about 22.22.3; exits 0.

- [ ] **Step 7: Commit**

```bash
git add package.json pnpm-workspace.yaml .npmrc .gitignore .editorconfig \
  tsconfig.base.json tsconfig.json vitest.config.ts scripts/ LICENSE README.md THIRD_PARTY_NOTICES.md
git commit -m "chore: repository skeleton, toolchain check and licence"
```

---

### Task 2: `@nativeangular/core` — the backend SPI

**Files:**
- Create: `packages/core/package.json`, `packages/core/tsconfig.json`
- Create: `packages/core/src/spi/element-kind.ts`
- Create: `packages/core/src/spi/document.ts`
- Create: `packages/core/src/spi/capability.ts`
- Create: `packages/core/src/spi/element-factory.ts`
- Create: `packages/core/src/spi/backend.ts`
- Create: `packages/core/src/registry/element-registry.ts`
- Create: `packages/core/src/index.ts`
- Test: `packages/core/src/registry/element-registry.spec.ts`

**Interfaces:**
- Consumes: nothing from Task 1 except the workspace and `tsconfig.base.json`.
- Produces (imported by Tasks 3–5):
  - `type ElementKind = 'view' | 'text' | 'image' | 'scrollView' | 'list' | 'textInput' | 'pressable' | 'switch' | 'activityIndicator' | 'slot'`
  - `type PlatformId = 'ios' | 'android' | 'watchos' | 'tvos' | 'visionos' | 'macos' | 'wearos'`
  - `type CapabilityLevel = 'full' | 'partial' | 'none'`
  - `interface CapabilityDescriptor { feature: string; level: CapabilityLevel }`
  - `interface ElementContext { platform: PlatformId; injector: Injector }`
  - `interface NativeElementHandle { readonly id: string; readonly kind: ElementKind }`
  - `interface NativeElementFactory { readonly kind: ElementKind; readonly capability: CapabilityDescriptor; create(ctx): NativeElementHandle; setProp(handle, name, value): void; setStyle(handle, style): void; append(handle, child): void; remove(handle): void; dispose(handle): void }`
  - `interface NativeDocument { createElement(tag: string): unknown; createTextNode(text: string): unknown; createComment(text: string): unknown; body: unknown; head: unknown; documentElement: unknown; querySelector(selector: string): unknown; querySelectorAll(selector: string): unknown[]; addEventListener(type: string, listener: (event: unknown) => void): void; removeEventListener(type: string, listener: (event: unknown) => void): void }`
  - `interface NativeBackend { readonly id: string; readonly platforms: readonly PlatformId[]; readonly capabilities: Readonly<Record<string, CapabilityLevel>>; readonly elements: readonly NativeElementFactory[]; createRendererFactory(): RendererFactory2; createDocument(): NativeDocument }`
  - `class ElementRegistry { register(factory: NativeElementFactory): void; resolve(kind: ElementKind): NativeElementFactory; kinds(): ElementKind[] }`

- [ ] **Step 1: Write the failing registry spec**

Create `packages/core/src/registry/element-registry.spec.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { ElementRegistry } from './element-registry';
import type { NativeElementFactory } from '../spi/element-factory';

function fakeFactory(kind: 'view' | 'text'): NativeElementFactory {
  return {
    kind,
    capability: { feature: kind, level: 'full' },
    create: () => ({ id: `${kind}-1`, kind }),
    setProp: () => {},
    setStyle: () => {},
    append: () => {},
    remove: () => {},
    dispose: () => {},
  };
}

describe('ElementRegistry', () => {
  it('resolves a registered element kind', () => {
    const registry = new ElementRegistry();
    const factory = fakeFactory('view');
    registry.register(factory);

    expect(registry.resolve('view')).toBe(factory);
    expect(registry.kinds()).toEqual(['view']);
  });

  it('throws a typed error for an unknown kind', () => {
    const registry = new ElementRegistry();

    expect(() => registry.resolve('text')).toThrowError(
      /No element implementation registered for "text"/,
    );
  });

  it('replaces a factory registered twice for the same kind', () => {
    const registry = new ElementRegistry();
    const first = fakeFactory('view');
    const second = fakeFactory('view');

    registry.register(first);
    registry.register(second);

    expect(registry.resolve('view')).toBe(second);
    expect(registry.kinds()).toEqual(['view']);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm vitest run packages/core/src/registry/element-registry.spec.ts`
Expected: FAIL — `Failed to resolve import "./element-registry"`.

- [ ] **Step 3: Write the SPI types**

`packages/core/src/spi/element-kind.ts`:

```ts
/** The abstract element kinds every backend must be able to provide. */
export type ElementKind =
  | 'view'
  | 'text'
  | 'image'
  | 'scrollView'
  | 'list'
  | 'textInput'
  | 'pressable'
  | 'switch'
  | 'activityIndicator'
  | 'slot';

export const ELEMENT_KINDS: readonly ElementKind[] = [
  'view',
  'text',
  'image',
  'scrollView',
  'list',
  'textInput',
  'pressable',
  'switch',
  'activityIndicator',
  'slot',
];
```

`packages/core/src/spi/capability.ts`:

```ts
export type PlatformId =
  | 'ios'
  | 'android'
  | 'watchos'
  | 'tvos'
  | 'visionos'
  | 'macos'
  | 'wearos';

export type CapabilityLevel = 'full' | 'partial' | 'none';

export interface CapabilityDescriptor {
  /** Dotted feature name, e.g. `textInput.secure`. */
  readonly feature: string;
  readonly level: CapabilityLevel;
}
```

`packages/core/src/spi/element-factory.ts`:

```ts
import type { Injector } from '@angular/core';
import type { ElementKind } from './element-kind';
import type { CapabilityDescriptor, PlatformId } from './capability';

export interface ElementContext {
  readonly platform: PlatformId;
  readonly injector: Injector;
}

export interface NativeElementHandle {
  readonly id: string;
  readonly kind: ElementKind;
}

/** Backend-neutral style payload produced by the style compiler. */
export type NativeStyle = Readonly<Record<string, string | number>>;

export interface NativeElementFactory {
  readonly kind: ElementKind;
  readonly capability: CapabilityDescriptor;
  create(ctx: ElementContext): NativeElementHandle;
  setProp(handle: NativeElementHandle, name: string, value: unknown): void;
  setStyle(handle: NativeElementHandle, style: NativeStyle): void;
  append(handle: NativeElementHandle, child: NativeElementHandle): void;
  remove(handle: NativeElementHandle): void;
  dispose(handle: NativeElementHandle): void;
}
```

`packages/core/src/spi/document.ts`:

```ts
/**
 * The minimum document surface Angular 22 touches. Backends supply their own
 * implementation; the platform only needs `createElement` for the root host and
 * `body` for a few core code paths that probe the document.
 */
export interface NativeDocument {
  createElement(tag: string): unknown;
  createTextNode(text: string): unknown;
  createComment(text: string): unknown;
  body: unknown;
  head: unknown;
  documentElement: unknown;
  querySelector(selector: string): unknown;
  querySelectorAll(selector: string): unknown[];
  addEventListener(type: string, listener: (event: unknown) => void): void;
  removeEventListener(type: string, listener: (event: unknown) => void): void;
}
```

`packages/core/src/spi/backend.ts`:

```ts
import type { RendererFactory2 } from '@angular/core';
import type { CapabilityLevel, PlatformId } from './capability';
import type { NativeElementFactory } from './element-factory';
import type { NativeDocument } from './document';

export interface NativeBackend {
  /** Stable identifier, e.g. `fabric` or `rust`. */
  readonly id: string;
  readonly platforms: readonly PlatformId[];
  /** Feature → level. M1 ships declarations only; the resolver lands in M2. */
  readonly capabilities: Readonly<Record<string, CapabilityLevel>>;
  readonly elements: readonly NativeElementFactory[];
  createRendererFactory(): RendererFactory2;
  /** The document shim this backend renders into. */
  createDocument(): NativeDocument;
}
```

- [ ] **Step 4: Write the registry**

`packages/core/src/registry/element-registry.ts`:

```ts
import type { ElementKind } from '../spi/element-kind';
import type { NativeElementFactory } from '../spi/element-factory';

export class ElementRegistry {
  private readonly factories = new Map<ElementKind, NativeElementFactory>();

  register(factory: NativeElementFactory): void {
    this.factories.set(factory.kind, factory);
  }

  resolve(kind: ElementKind): NativeElementFactory {
    const factory = this.factories.get(kind);
    if (!factory) {
      throw new Error(
        `No element implementation registered for "${kind}". ` +
          'Register one with ElementRegistry.register() before bootstrapping.',
      );
    }
    return factory;
  }

  kinds(): ElementKind[] {
    return [...this.factories.keys()];
  }
}
```

`packages/core/src/index.ts`:

```ts
export * from './spi/element-kind';
export * from './spi/capability';
export * from './spi/document';
export * from './spi/element-factory';
export * from './spi/backend';
export * from './registry/element-registry';
```

- [ ] **Step 5: Add the package manifest and tsconfig**

`packages/core/package.json`:

```json
{
  "name": "@nativeangular/core",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "license": "MIT",
  "peerDependencies": { "@angular/core": "^22.2.0" },
  "exports": { ".": "./src/index.ts" }
}
```

`packages/core/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": { "outDir": "./dist", "rootDir": "./src" },
  "include": ["src/**/*.ts"]
}
```

- [ ] **Step 6: Register the package in the root solution file**

In the root `tsconfig.json`, add the core reference:

```json
"references": [{ "path": "./packages/core" }]
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `pnpm vitest run packages/core/src/registry/element-registry.spec.ts`
Expected: 3 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add packages/core tsconfig.json
git commit -m "feat(core): backend SPI, element kinds and the element registry"
```

---

### Task 3: `@nativeangular/platform` — the DOM-less Angular platform

**Files:**
- Create: `packages/platform/package.json`, `packages/platform/tsconfig.json`
- Create: `packages/platform/src/angular-internals.ts`
- Create: `packages/platform/src/document.ts`
- Create: `packages/platform/src/providers.ts`
- Create: `packages/platform/src/bootstrap.ts`
- Create: `packages/platform/src/index.ts`
- Test: `packages/platform/src/bootstrap.spec.ts`

**Interfaces:**
- Consumes: `@nativeangular/core` — `NativeBackend` (Task 2).
- Produces:
  - `interface NativeDocument { createElement(tag: string): unknown; createTextNode(text: string): unknown; createComment(text: string): unknown; body: unknown; head: unknown; documentElement: unknown; querySelector(selector: string): unknown; querySelectorAll(selector: string): unknown[]; addEventListener(...): void; removeEventListener(...): void }`
  - `provideNativeAngular(backend: NativeBackend, document?: NativeDocument): EnvironmentProviders`
  - `bootstrapNativeAngular(rootComponent: Type<unknown>): Promise<ApplicationRef>`
  - `NATIVE_PLATFORM_ID` — `'nativeangular'`

- [ ] **Step 1: Write the failing bootstrap spec**

Create `packages/platform/src/bootstrap.spec.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { DOCUMENT, RendererFactory2, signal } from '@angular/core';
import type { NativeBackend, NativeDocument } from '@nativeangular/core';
import { defineNativeComponent } from './angular-internals';
import { bootstrapNativeAngular } from './bootstrap';

/** The smallest possible backend: a node type and a renderer that creates them. */
class StubNode {
  constructor(readonly nodeName: string) {}
}

/** One call the custom renderer received, recorded so the spec can assert it. */
interface RendererCall {
  readonly method: string;
  readonly argument?: string;
}

interface StubBackend {
  /** The backend handed to `bootstrapNativeAngular`. */
  readonly backend: NativeBackend;
  /** The document this backend supplies. */
  readonly document: NativeDocument;
  /** The renderer factory this backend supplies. */
  readonly rendererFactory: RendererFactory2;
  /** Every call the custom renderer received, in order. */
  readonly calls: RendererCall[];
}

/**
 * Builds a backend whose renderer records every call it receives, so the
 * spec can prove that the application really rendered through it.
 */
function createStubBackend(id: string): StubBackend {
  const calls: RendererCall[] = [];

  const document: NativeDocument = {
    createElement: (tag) => new StubNode(tag),
    createTextNode: () => new StubNode('#text'),
    createComment: () => new StubNode('#comment'),
    body: new StubNode('body'),
    head: new StubNode('head'),
    documentElement: new StubNode('html'),
    querySelector: () => null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  const rendererFactory: RendererFactory2 = {
    createRenderer: () => ({
      data: {},
      createElement: (name: string) => {
        calls.push({ method: 'createElement', argument: name });
        return new StubNode(name);
      },
      createComment: () => new StubNode('#comment'),
      createText: () => new StubNode('#text'),
      appendChild: () => {},
      insertBefore: () => {},
      removeChild: () => {},
      selectRootElement: (selector: string) => {
        calls.push({ method: 'selectRootElement', argument: selector });
        return new StubNode(selector);
      },
      parentNode: () => null,
      nextSibling: () => null,
      destroyNode: null,
      setAttribute: (_element, name) => {
        calls.push({ method: 'setAttribute', argument: name });
      },
      removeAttribute: () => {},
      addClass: () => {},
      removeClass: () => {},
      setStyle: () => {},
      removeStyle: () => {},
      setProperty: () => {},
      setValue: () => {},
      listen: () => () => {},
      destroy: () => {},
    }),
    begin: () => {},
    end: () => {},
  };

  const backend: NativeBackend = {
    id,
    platforms: ['ios'],
    capabilities: {},
    elements: [],
    createDocument: () => document,
    createRendererFactory: () => rendererFactory,
  };

  return { backend, document, rendererFactory, calls };
}

class RootComponent {
  readonly value = signal('hello');
}

/**
 * The suite is AOT-only: `@angular/compiler` may not be imported, so the
 * component is declared in the form `ngc` emits. The template is deliberately
 * the plain `<an-view />`: static attributes, bindings and text need the
 * `consts` table the AOT compiler emits, so they are asserted by the
 * compiled fixture in Task 5, not here.
 */
defineNativeComponent(RootComponent, {
  selectors: [['an-root']],
  elements: ['an-view'],
});

describe('bootstrapNativeAngular', () => {
  it('bootstraps an application without a DOM and exposes the root component', async () => {
    const stub = createStubBackend('stub');
    const appRef = await bootstrapNativeAngular(RootComponent, stub.backend);
    await appRef.whenStable();

    expect(appRef.components.length).toBe(1);
    expect(appRef.components[0]!.instance).toBeInstanceOf(RootComponent);

    appRef.destroy();
  });

  it('renders the template through the custom renderer', async () => {
    const stub = createStubBackend('stub');
    const appRef = await bootstrapNativeAngular(RootComponent, stub.backend);
    await appRef.whenStable();

    // The DOM-less render path: the platform asked the renderer for the root
    // host and the renderer created the element the template declares. A
    // template that rendered nothing would fail here.
    expect(stub.calls).toContainEqual({ method: 'selectRootElement', argument: 'an-root' });
    expect(stub.calls).toContainEqual({ method: 'createElement', argument: 'an-view' });

    appRef.destroy();
  });

  it('gives every bootstrap its own platform injector', async () => {
    const first = createStubBackend('first');
    const second = createStubBackend('second');

    const firstApp = await bootstrapNativeAngular(RootComponent, first.backend);
    await firstApp.whenStable();

    // The second bootstrap owns a new platform — Angular allows one platform
    // per realm, so the platform of the first application is released here.
    const secondApp = await bootstrapNativeAngular(RootComponent, second.backend);

    // The second application must resolve the platform providers of its own
    // backend — never the ones the first application installed.
    expect(secondApp.injector.get(RendererFactory2)).toBe(second.rendererFactory);
    expect(secondApp.injector.get(DOCUMENT)).toBe(second.document);
    expect(secondApp.injector.get(RendererFactory2)).not.toBe(first.rendererFactory);
    expect(secondApp.injector.get(DOCUMENT)).not.toBe(first.document);

    secondApp.destroy();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm vitest run packages/platform/src/bootstrap.spec.ts`
Expected: FAIL — `Failed to resolve import "./bootstrap"`.

- [ ] **Step 3: Write the Angular internals adapter — the only file allowed to touch `ɵ` API**

`packages/platform/src/angular-internals.ts`:

```ts
/**
 * The ONLY file in this repository that may import Angular private (`ɵ`) API.
 *
 * | Import                     | Why it is needed                                              |
 * |----------------------------|---------------------------------------------------------------|
 * | `ɵinternalCreateApplication` | Angular 22 ships `bootstrapApplication` only in `@angular/platform-browser`, which we ban. This is the same entry point NativeScript's Angular integration uses. |
 * | `ɵsetDocument`              | Core resolves the root element through a module-level document reference, not through the `DOCUMENT` injection token. Without it, `getDocument()` throws NG0210. |
 * | `ɵINJECTOR_SCOPE`           | Makes the application injector root-scoped so root-provided token factories (e.g. the internal error handler) resolve. |
 * | `ɵIMAGE_CONFIG`             | Angular 22 exports the image configuration token only under its `ɵ` name. The adapter re-exports it as `IMAGE_CONFIG`. |
 * | `ɵɵdefineComponent`         | Declares a component in the AOT form `ngc` emits. The suite is AOT-only — `@angular/compiler` may not be imported — so a `@Component({ template })` decorator cannot be compiled at runtime. |
 * | `ɵɵelementStart`, `ɵɵelementEnd` | Emit the creation instructions of that AOT component's template. |
 * | `RENDER_FLAGS_CREATE`       | A hand-written copy of `RenderFlags.Create` (value `1`). Angular declares `RenderFlags` as an ambient const enum, so it has no runtime value and cannot be imported. The value is pinned to Angular 22.2.x. |
 * | `createPlatformFactory`, `platformCore` | Public API, but version-coupled: this is the recipe that builds the DOM-less platform injector. Angular allows one platform per realm, so the factory is invoked per bootstrap — see `createNativePlatform`. |
 *
 * Every entry above is pinned to Angular 22.2.x. Re-verify this table on every
 * Angular major upgrade — see `docs/architecture/m0-gap-analysis.md`.
 */
import {
  PlatformRef,
  createPlatformFactory,
  platformCore,
  ɵIMAGE_CONFIG,
  ɵINJECTOR_SCOPE,
  ɵinternalCreateApplication,
  ɵsetDocument,
  ɵɵdefineComponent,
  ɵɵelementEnd,
  ɵɵelementStart,
} from '@angular/core';
import type { Provider, StaticProvider } from '@angular/core';

export const ANGULAR_INTERNALS_SCOPE = 'root';

/**
 * Angular 22 exports the image configuration token only under its `ɵ` name.
 * The adapter re-exports it under a plain name so that no other file has to
 * touch `ɵ` API.
 */
export { ɵIMAGE_CONFIG as IMAGE_CONFIG };

/**
 * `RenderFlags.Create`. Angular declares `RenderFlags` as an ambient const
 * enum, so it has no runtime value and cannot be imported.
 */
const RENDER_FLAGS_CREATE = 1;

export function setNativeDocument(document: unknown): void {
  ɵsetDocument(document as never);
}

export function rootScopeProvider(): { provide: typeof ɵINJECTOR_SCOPE; useValue: string } {
  return { provide: ɵINJECTOR_SCOPE, useValue: ANGULAR_INTERNALS_SCOPE };
}

/**
 * The platform of the application that was bootstrapped last.
 *
 * Angular allows exactly one platform per realm: `createPlatform` throws
 * NG0400 when a second one is created, and `createPlatformFactory` *reuses*
 * the platform that already exists. Without this handle a second
 * `bootstrapNativeAngular` in the same realm would resolve `DOCUMENT` and
 * `RendererFactory2` from the *first* application's platform injector, while
 * `ɵsetDocument` had already pointed the module-level document at the second
 * application — an inconsistent state no test would catch.
 */
let activePlatform: PlatformRef | undefined;

/**
 * Creates the platform that owns one application's platform-level providers.
 *
 * The previous platform is destroyed first, so every application gets its own
 * platform injector and no platform is shared between two applications.
 * Angular registers every application as a destroy listener of its platform,
 * so replacing the platform also tears down the application that owned it —
 * the same "one platform per realm" contract Angular itself enforces.
 */
export function createNativePlatform(platformProviders: Provider[]): PlatformRef {
  if (activePlatform && !activePlatform.destroyed) {
    activePlatform.destroy();
  }

  activePlatform = createPlatformFactory(
    platformCore,
    'nativeangular',
    platformProviders as StaticProvider[],
  )();

  return activePlatform;
}

export async function createNativeApplication(config: {
  rootComponent: unknown;
  appProviders: unknown[];
  platformRef: PlatformRef;
}): Promise<unknown> {
  return ɵinternalCreateApplication({
    rootComponent: config.rootComponent as never,
    appProviders: config.appProviders as never,
    // Angular ignores `platformProviders` when a `platformRef` is supplied:
    // the platform injector already carries them.
    platformProviders: [],
    platformRef: config.platformRef,
  });
}

export interface NativeComponentDefinition {
  /** The element selectors, e.g. `[['an-root']]`. */
  readonly selectors: readonly (readonly string[])[];
  /** The static child elements the template creates, in order. */
  readonly elements?: readonly string[];
}

/**
 * Declares a standalone component the way the AOT compiler emits it.
 *
 * The M0 suite is AOT-only: `@angular/compiler` must never be imported, so a
 * `@Component({ template })` decorator cannot be compiled at runtime — Angular
 * would demand the JIT compiler. This helper writes the same definition by
 * hand: a factory, a standalone component definition and the template
 * instructions that create every static element.
 *
 * Only static element *names* are supported. Static attributes, bindings and
 * text need the `consts` table `ngc` emits, so they are covered by the
 * compiled fixture in Task 5 — not here.
 */
export function defineNativeComponent<T extends object>(
  type: new () => T,
  definition: NativeComponentDefinition,
): void {
  const elements = definition.elements ?? [];
  const target = type as unknown as { ɵfac?: unknown; ɵcmp?: unknown };

  target.ɵfac = function (override?: unknown): T {
    return new ((override as (new () => T) | undefined) ?? type)();
  };

  target.ɵcmp = ɵɵdefineComponent({
    type,
    selectors: definition.selectors as (string | number)[][],
    standalone: true,
    decls: elements.length,
    vars: 0,
    template: (flags, _context) => {
      if (flags & RENDER_FLAGS_CREATE) {
        elements.forEach((name, index) => {
          ɵɵelementStart(index, name);
          ɵɵelementEnd();
        });
      }
    },
  });
}
```

- [ ] **Step 4: Write the document shim**

`packages/platform/src/document.ts`:

```ts
import type { NativeDocument } from '@nativeangular/core';

export type { NativeDocument };

/**
 * Creates a document whose elements are produced by the backend's own element
 * factory. `createElement` is delegated so the root host is a real backend node.
 */
export function createNativeDocument(createElement: (tag: string) => unknown): NativeDocument {
  return {
    createElement,
    createTextNode: (text: string) => createElement('#text'),
    createComment: (text: string) => createElement('#comment'),
    body: createElement('body'),
    head: createElement('head'),
    documentElement: createElement('html'),
    querySelector: () => null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    removeEventListener: () => {},
  };
}
```

- [ ] **Step 5: Write the provider sets**

`packages/platform/src/providers.ts`:

```ts
import {
  APP_ID,
  CSP_NONCE,
  DOCUMENT,
  ErrorHandler,
  PLATFORM_ID,
  RendererFactory2,
  makeEnvironmentProviders,
} from '@angular/core';
import type { EnvironmentProviders, Provider } from '@angular/core';
import type { NativeBackend, NativeDocument } from '@nativeangular/core';
import { IMAGE_CONFIG, rootScopeProvider } from './angular-internals';

export const NATIVE_PLATFORM_ID = 'nativeangular';

/**
 * Platform-level providers. These live in the *platform* injector.
 *
 * `CSP_NONCE: null` stops the token's default factory from reading
 * `DOCUMENT.body.querySelector()`. `IMAGE_CONFIG` disables Angular's dev-mode
 * image warnings, which scan for `<img>` elements in a DOM we do not have.
 */
export function platformProviders(
  backend: NativeBackend,
  document: NativeDocument,
): Provider[] {
  return [
    { provide: PLATFORM_ID, useValue: NATIVE_PLATFORM_ID },
    { provide: DOCUMENT, useValue: document },
    { provide: RendererFactory2, useValue: backend.createRendererFactory() },
    { provide: CSP_NONCE, useValue: null },
    {
      provide: IMAGE_CONFIG,
      useValue: { disableImageSizeWarning: true, disableImageLazyLoadWarning: true },
    },
  ];
}

/**
 * Application-level providers. These live in the *application* injector.
 *
 * `ɵINJECTOR_SCOPE: 'root'` and `ErrorHandler` MUST be here and not in the
 * platform providers: in the platform injector the internal error handler
 * resolves circularly (verified — see the M0 gap analysis).
 */
export function appProviders(): Provider[] {
  return [
    rootScopeProvider(),
    { provide: ErrorHandler, useClass: ErrorHandler },
    { provide: APP_ID, useValue: NATIVE_PLATFORM_ID },
  ];
}

/**
 * The native platform's provider set as `EnvironmentProviders`, for
 * applications that compose their own bootstrap. `bootstrapNativeAngular`
 * installs the same providers.
 */
export function provideNativeAngular(
  backend: NativeBackend,
  document: NativeDocument = backend.createDocument(),
): EnvironmentProviders {
  return makeEnvironmentProviders(platformProviders(backend, document));
}
```

- [ ] **Step 6: Write the bootstrap entry point**

`packages/platform/src/bootstrap.ts`:

```ts
import { ApplicationRef, type Type } from '@angular/core';
import type { NativeBackend } from '@nativeangular/core';
import {
  createNativeApplication,
  createNativePlatform,
  setNativeDocument,
} from './angular-internals';
import type { NativeDocument } from './document';
import { appProviders, platformProviders } from './providers';

export interface NativeAngularOptions {
  /** The document shim. Defaults to the backend's own document. */
  document?: NativeDocument;
}

/**
 * Bootstraps an Angular application on top of a native backend, with no DOM.
 *
 * Every call owns its platform: the platform injector holds this backend's
 * `DOCUMENT` and `RendererFactory2`, so a second bootstrap in the same realm
 * never inherits the first application's platform providers.
 */
export async function bootstrapNativeAngular(
  rootComponent: Type<unknown>,
  backend: NativeBackend,
  options: NativeAngularOptions = {},
): Promise<ApplicationRef> {
  const document = options.document ?? backend.createDocument();

  setNativeDocument(document);

  const platformRef = createNativePlatform(platformProviders(backend, document));

  const appRef = (await createNativeApplication({
    rootComponent,
    appProviders: appProviders(),
    platformRef,
  })) as ApplicationRef;

  return appRef;
}
```

> **Implementation note:** the default document above is deliberately minimal. Task 4 supplies the real fake document, because a backend's element creation must go through its renderer, not through a throwaway factory instance. If Task 4's fake backend exposes a `createDocument()` helper, replace the default with it — do not leave two document implementations in the tree.

- [ ] **Step 7: Add the package manifest, tsconfig and barrel**

`packages/platform/package.json`:

```json
{
  "name": "@nativeangular/platform",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "license": "MIT",
  "peerDependencies": { "@angular/core": "22.2.5" },
  "dependencies": { "@nativeangular/core": "workspace:*" },
  "exports": { ".": "./src/index.ts" }
}
```

`packages/platform/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": { "outDir": "./dist", "rootDir": "./src" },
  "include": ["src/**/*.ts"],
  "references": [{ "path": "../core" }]
}
```

`packages/platform/src/index.ts`:

```ts
export * from './document';
export * from './providers';
export * from './bootstrap';
```

- [ ] **Step 8: Register the package in the root solution file**

In the root `tsconfig.json`, add the platform reference:

```json
"references": [{ "path": "./packages/core" }, { "path": "./packages/platform" }]
```

- [ ] **Step 9: Run the test to verify it passes**

Run: `pnpm vitest run packages/platform/src/bootstrap.spec.ts`
Expected: 1 test PASS — the application bootstraps with no DOM present.

> If the test fails with `NG0402 … BrowserModule is imported`, the `ErrorHandler` provider is missing from the **app** providers. If it fails with a circular dependency on `internal error handler`, `ɵINJECTOR_SCOPE` is in the platform providers instead of the app providers. Both were measured during the spike.

- [ ] **Step 10: Commit**

```bash
git add packages/platform tsconfig.json
git commit -m "feat(platform): DOM-less Angular platform, document shim and bootstrap"
```

---

### Task 4: `@nativeangular/testing` — the fake backend

**Files:**
- Create: `packages/testing/package.json`, `packages/testing/tsconfig.json`
- Create: `packages/testing/src/fake-node.ts`
- Create: `packages/testing/src/fake-renderer.ts`
- Create: `packages/testing/src/fake-backend.ts`
- Create: `packages/testing/src/queries.ts`
- Create: `packages/testing/src/index.ts`
- Test: `packages/testing/src/fake-renderer.spec.ts`

**Interfaces:**
- Consumes: `@nativeangular/core` (Task 2), `@nativeangular/platform` (Task 3).
- Produces:
  - `class FakeNode` — the DOM-less node model (see below).
  - `class FakeRenderer implements Renderer2` — implements the measured Angular 22 renderer contract.
  - `class FakeRendererFactory implements RendererFactory2` — records every host element.
  - `createFakeBackend(): NativeBackend & { document: NativeDocument; factory: FakeRendererFactory }`
  - `queryByText(root: FakeNode, text: string): FakeNode | null`
  - `queryAll(root: FakeNode, tagName: string): FakeNode[]`

- [ ] **Step 1: Write the failing renderer spec**

Create `packages/testing/src/fake-renderer.spec.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { FakeRenderer } from './fake-renderer';
import { FakeNode } from './fake-node';

describe('FakeRenderer — the measured Angular 22 renderer contract', () => {
  it('tolerates createRenderer(null, null)', () => {
    const renderer = new FakeRenderer(null);
    expect(renderer.createElement('an-view')).toBeInstanceOf(FakeNode);
  });

  it('removes a child when Angular passes a null parent (F5)', () => {
    const renderer = new FakeRenderer(null);
    const parent = renderer.createElement('an-view');
    const child = renderer.createElement('an-text');

    renderer.appendChild(parent, child);
    expect(parent.childNodes).toHaveLength(1);

    renderer.removeChild(null, child);
    expect(parent.childNodes).toHaveLength(0);
    expect(child.parentNode).toBeNull();
  });

  it('clears children when Angular sets textContent to an empty string (F6)', () => {
    const renderer = new FakeRenderer(null);
    const parent = renderer.createElement('an-view');
    renderer.appendChild(parent, renderer.createElement('an-text'));

    parent.textContent = '';

    expect(parent.childNodes).toHaveLength(0);
  });

  it('stores classes as the class attribute, because Angular writes them that way (F7)', () => {
    const renderer = new FakeRenderer(null);
    const el = renderer.createElement('an-view');

    renderer.setAttribute(el, 'class', 'root selected');

    expect(el.getAttribute('class')).toBe('root selected');
    expect(el.classList).toEqual(['root', 'selected']);
  });

  it('writes text through nodeValue (F8)', () => {
    const renderer = new FakeRenderer(null);
    const text = renderer.createText('hello');

    renderer.setValue(text, 'updated');

    expect(text.nodeValue).toBe('updated');
  });

  it('reports parent and next sibling (F9)', () => {
    const renderer = new FakeRenderer(null);
    const parent = renderer.createElement('an-view');
    const first = renderer.createElement('an-text');
    const second = renderer.createElement('an-image');

    renderer.appendChild(parent, first);
    renderer.appendChild(parent, second);

    expect(first.parentNode).toBe(parent);
    expect(first.nextSibling).toBe(second);
    expect(second.nextSibling).toBeNull();
  });

  it('rejects a property write for an unknown property instead of silently ignoring it (F10)', () => {
    const renderer = new FakeRenderer(null);
    const el = renderer.createElement('an-view');

    expect(() => renderer.setProperty(el, 'label', 'hello')).toThrowError(
      /Unknown property "label" on <an-view>/,
    );
  });

  it('accepts a property write for a declared property', () => {
    const renderer = new FakeRenderer(null, { knownProperties: ['label'] });
    const el = renderer.createElement('an-view');

    renderer.setProperty(el, 'label', 'hello');

    expect(el.properties.label).toBe('hello');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm vitest run packages/testing/src/fake-renderer.spec.ts`
Expected: FAIL — `Failed to resolve import "./fake-renderer"`.

- [ ] **Step 3: Write the node model**

`packages/testing/src/fake-node.ts`:

```ts
export class FakeText {
  nodeValue: string;
  constructor(value: string) {
    this.nodeValue = value;
  }
  get textContent(): string {
    return this.nodeValue;
  }
  set textContent(value: string) {
    this.nodeValue = value;
  }
}

export class FakeNode {
  readonly childNodes: Array<FakeNode | FakeText> = [];
  readonly attributes: Record<string, string> = {};
  readonly styles: Record<string, string> = {};
  readonly properties: Record<string, unknown> = {};
  readonly listeners: Record<string, (event: unknown) => void> = {};

  parentNode: FakeNode | null = null;

  constructor(readonly nodeName: string) {}

  get classList(): string[] {
    return (this.attributes['class'] ?? '').split(/\s+/).filter(Boolean);
  }

  getAttribute(name: string): string | null {
    return this.attributes[name] ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.attributes[name] = value;
  }

  removeAttribute(name: string): void {
    delete this.attributes[name];
  }

  get textContent(): string {
    return this.childNodes
      .map((child) => (child instanceof FakeText ? child.nodeValue : child.textContent))
      .join('');
  }

  set textContent(value: string) {
    for (const child of this.childNodes) {
      if (child instanceof FakeNode) child.parentNode = null;
    }
    this.childNodes.length = 0;
    if (value !== '') this.childNodes.push(new FakeText(value));
  }

  get nextSibling(): FakeNode | FakeText | null {
    if (!this.parentNode) return null;
    const siblings = this.parentNode.childNodes;
    const index = siblings.indexOf(this);
    return index >= 0 ? (siblings[index + 1] ?? null) : null;
  }

  appendChild(child: FakeNode | FakeText): void {
    this.childNodes.push(child);
    if (child instanceof FakeNode) child.parentNode = this;
  }

  insertBefore(child: FakeNode | FakeText, ref: FakeNode | FakeText): void {
    const index = this.childNodes.indexOf(ref);
    this.childNodes.splice(index < 0 ? this.childNodes.length : index, 0, child);
    if (child instanceof FakeNode) child.parentNode = this;
  }

  removeChild(child: FakeNode | FakeText): void {
    const index = this.childNodes.indexOf(child);
    if (index >= 0) this.childNodes.splice(index, 1);
    if (child instanceof FakeNode) child.parentNode = null;
  }
}
```

- [ ] **Step 4: Write the renderer**

`packages/testing/src/fake-renderer.ts`:

```ts
import type { Renderer2 } from '@angular/core';
import { FakeNode, FakeText } from './fake-node';

export interface FakeRendererOptions {
  /**
   * Property names the host element accepts. A write to anything else is a
   * build-time mistake that would otherwise be swallowed on a device, where
   * `setProperty` has no meaning — see finding F10.
   */
  knownProperties?: readonly string[];
}

export class FakeRenderer implements Renderer2 {
  readonly data: Record<string, unknown> = {};

  constructor(
    private readonly host: FakeNode | null,
    private readonly options: FakeRendererOptions = {},
  ) {}

  createElement(name: string): FakeNode {
    return new FakeNode(name);
  }
  createComment(value: string): FakeText {
    return new FakeText(value);
  }
  createText(value: string): FakeText {
    return new FakeText(value);
  }

  appendChild(parent: FakeNode, child: FakeNode | FakeText): void {
    parent.appendChild(child);
  }
  insertBefore(parent: FakeNode, child: FakeNode | FakeText, ref: FakeNode | FakeText): void {
    parent.insertBefore(child, ref);
  }

  /**
   * Angular 22 calls `removeChild(null, child, …)` — the parent argument is
   * null and the renderer must resolve the parent itself (finding F5).
   */
  removeChild(parent: FakeNode | null, child: FakeNode | FakeText): void {
    const actualParent = parent ?? (child instanceof FakeNode ? child.parentNode : null);
    actualParent?.removeChild(child);
  }

  selectRootElement(selector: string): FakeNode {
    return new FakeNode(selector);
  }

  setAttribute(el: FakeNode, name: string, value: string): void {
    el.setAttribute(name, value);
  }
  removeAttribute(el: FakeNode, name: string): void {
    el.removeAttribute(name);
  }

  addClass(el: FakeNode, name: string): void {
    const classes = new Set(el.classList);
    classes.add(name);
    el.setAttribute('class', [...classes].join(' '));
  }
  removeClass(el: FakeNode, name: string): void {
    const classes = new Set(el.classList);
    classes.delete(name);
    el.setAttribute('class', [...classes].join(' '));
  }

  setStyle(el: FakeNode, style: string, value: string): void {
    el.styles[style] = value;
  }
  removeStyle(el: FakeNode, style: string): void {
    delete el.styles[style];
  }

  /**
   * A native host has no DOM properties. An unknown property means the input
   * was never registered — in JIT that is exactly what happens with signal
   * inputs (finding F10). Failing loudly here turns a silent device bug into a
   * test failure.
   */
  setProperty(el: FakeNode, name: string, value: unknown): void {
    const known = this.options.knownProperties;
    if (known && !known.includes(name)) {
      throw new Error(
        `Unknown property "${name}" on <${el.nodeName}>. ` +
          'Known properties: ' + (known.length ? known.join(', ') : '(none declared)'),
      );
    }
    el.properties[name] = value;
  }

  setValue(node: FakeNode | FakeText, value: string): void {
    if (node instanceof FakeText) node.nodeValue = value;
  }

  listen(target: FakeNode, event: string, callback: (event: unknown) => void): () => void {
    target.listeners[event] = callback;
    return () => {
      delete target.listeners[event];
    };
  }

  /** Angular calls `renderer.destroy()` when a view is destroyed (finding F9). */
  destroy(): void {
    this.host?.childNodes.splice(0);
  }
}
```

- [ ] **Step 5: Write the fake backend, the queries and the barrel**

`packages/testing/src/fake-backend.ts`:

```ts
import type { RendererFactory2, RendererType2 } from '@angular/core';
import type { NativeBackend, PlatformId } from '@nativeangular/core';
import { createNativeDocument } from '@nativeangular/platform';
import type { NativeDocument } from '@nativeangular/core';
import { FakeNode } from './fake-node';
import { FakeRenderer } from './fake-renderer';

export class FakeRendererFactory implements RendererFactory2 {
  /** Every host Angular rendered into. The first non-null entry is the root. */
  readonly hosts: Array<FakeNode | null> = [];

  constructor(private readonly knownProperties: readonly string[] = []) {}

  createRenderer(hostElement: FakeNode | null, _type: RendererType2 | null): FakeRenderer {
    this.hosts.push(hostElement);
    return new FakeRenderer(hostElement, { knownProperties: this.knownProperties });
  }
  begin(): void {}
  end(): void {}
}

export interface FakeBackend extends NativeBackend {
  readonly factory: FakeRendererFactory;
  readonly document: NativeDocument;
  readonly knownProperties: readonly string[];
}

export function createFakeBackend(
  knownProperties: readonly string[] = [],
): FakeBackend {
  const factory = new FakeRendererFactory(knownProperties);
  const document = createNativeDocument((tag: string) => new FakeNode(tag));

  return {
    id: 'fake',
    platforms: ['ios', 'android'] as readonly PlatformId[],
    capabilities: {},
    elements: [],
    createRendererFactory: () => factory,
    createDocument: () => document,
    factory,
    document,
    knownProperties,
  };
}
```

`packages/testing/src/queries.ts`:

```ts
import { FakeNode, FakeText } from './fake-node';

export function queryAll(root: FakeNode, tagName: string): FakeNode[] {
  const found: FakeNode[] = [];
  const walk = (node: FakeNode): void => {
    for (const child of node.childNodes) {
      if (child instanceof FakeNode) {
        if (child.nodeName === tagName) found.push(child);
        walk(child);
      }
    }
  };
  walk(root);
  return found;
}

export function queryByText(root: FakeNode, text: string): FakeNode | null {
  let result: FakeNode | null = null;
  const walk = (node: FakeNode): void => {
    for (const child of node.childNodes) {
      if (child instanceof FakeText) {
        if (child.nodeValue === text && !result) result = node;
      } else if (child instanceof FakeNode) {
        walk(child);
      }
    }
  };
  walk(root);
  return result;
}

export function textOf(root: FakeNode): string {
  return root.textContent;
}
```

`packages/testing/src/index.ts`:

```ts
export * from './fake-node';
export * from './fake-renderer';
export * from './fake-backend';
export * from './queries';
```

`packages/testing/package.json`:

```json
{
  "name": "@nativeangular/testing",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "license": "MIT",
  "peerDependencies": { "@angular/core": "^22.2.0" },
  "dependencies": {
    "@nativeangular/core": "workspace:*",
    "@nativeangular/platform": "workspace:*"
  },
  "exports": { ".": "./src/index.ts" }
}
```

`packages/testing/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": { "outDir": "./dist", "rootDir": "./src" },
  "include": ["src/**/*.ts"],
  "references": [{ "path": "../core" }, { "path": "../platform" }]
}
```

- [ ] **Step 6: Register the package in the root solution file**

In the root `tsconfig.json`, add the testing reference:

```json
"references": [
  { "path": "./packages/core" },
  { "path": "./packages/platform" },
  { "path": "./packages/testing" }
]
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `pnpm vitest run packages/testing/src/fake-renderer.spec.ts`
Expected: 8 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add packages/testing tsconfig.json
git commit -m "feat(testing): DOM-less fake backend implementing the Angular 22 renderer contract"
```

---

### Task 5: AOT fixture pipeline and the end-to-end proof

**Files:**
- Create: `packages/testing/src/teardown.ts`
- Create: `packages/testing/fixtures/src/app.ts`
- Create: `packages/testing/fixtures/tsconfig.json`
- Create: `scripts/compile-fixtures.mjs`
- Modify: `packages/testing/src/index.ts` (export the teardown helper)
- Modify: `package.json` (compile the fixtures before the suite runs)
- Test: `packages/testing/src/aot.e2e.spec.ts`

**Interfaces:**
- Consumes: `@nativeangular/testing` (Task 4), `@nativeangular/platform` (Task 3), `@angular/core` (peer).
- Produces:
  - `pnpm run compile:fixtures` → writes `packages/testing/fixtures/out/app.js` (AOT ESM).
  - `packages/testing/fixtures/out/app.js` exports `AnRootComponent` and `AnViewComponent`.
  - `disposeNativeApplication(appRef: ApplicationRef): void` — destroys the application *and* the platform that owns it.

> **Background — one platform per realm.** Task 3 established that Angular allows exactly one platform per realm: `bootstrapNativeAngular` destroys the previous platform before creating its own, and Angular registers every application as a destroy listener of its platform. Two consequences for this task:
> 1. A second bootstrap in the same realm silently tears down the first application. A test that forgets to dispose leaves the *next* test with a half-destroyed app.
> 2. Disposing only the application leaves its platform alive. The teardown helper below destroys both, so every test starts from a clean realm.

- [ ] **Step 1: Write the failing end-to-end spec**

Create `packages/testing/src/aot.e2e.spec.ts`:

```ts
import { afterEach, beforeAll, describe, it, expect } from 'vitest';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { ApplicationRef, Type } from '@angular/core';
import { PlatformRef } from '@angular/core';
import { bootstrapNativeAngular } from '@nativeangular/platform';
import { createFakeBackend, disposeNativeApplication, queryAll, textOf } from './index';

// The compiled artefact is ESM emitted by `ngc`; it is imported dynamically so
// that a missing compilation produces a clear message instead of a module error.
const fixtureUrl = new URL('../fixtures/out/app.js', import.meta.url);

let AnRootComponent: Type<unknown> & { new (): { label: { set(value: string): void } } };

/**
 * Waits for Angular's scheduled change detection to run.
 *
 * `ChangeDetectionSchedulerImpl.notify()` registers a pending task when it
 * schedules a tick and removes it once the tick completes, and
 * `ApplicationRef.isStable` is derived from that same queue — so
 * `whenStable()` already waits for the scheduled tick. The extra macrotask is
 * belt and braces: it costs one event-loop turn and cannot fabricate a
 * re-render, so it cannot mask a "the view never re-rendered" bug.
 */
async function flush(appRef: ApplicationRef): Promise<void> {
  await appRef.whenStable();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

beforeAll(async () => {
  if (!existsSync(fileURLToPath(fixtureUrl))) {
    throw new Error(
      'AOT fixtures are missing. Run `pnpm run compile:fixtures` before the tests.',
    );
  }
  const module = await import(fixtureUrl.href);
  AnRootComponent = module.AnRootComponent;
});

describe('NativeAngular — AOT component on the fake backend', () => {
  let appRef: ApplicationRef | undefined;

  afterEach(() => {
    if (appRef) {
      disposeNativeApplication(appRef);
      appRef = undefined;
    }
  });

  it('renders the tree and binds a signal input to the child component', async () => {
    const backend = createFakeBackend(['label']);
    appRef = await bootstrapNativeAngular(AnRootComponent, backend, {
      document: backend.document,
    });
    await flush(appRef);

    const root = backend.factory.hosts.find((host) => host !== null)!;
    const views = queryAll(root, 'an-view');
    expect(views).toHaveLength(1);

    const view = views[0]!;
    // AOT registers the signal input, so the value arrives as a directive input.
    // It must never appear as a raw property write on the host element.
    expect(Object.keys(view.properties)).toEqual([]);
    expect(textOf(view)).toBe('hello');
    // Static attributes come from the compiler's `consts` table — the fidelity the
    // hand-written Task 3 definition deliberately does not cover.
    expect(view.getAttribute('class')).toBe('root');
  });

  it('re-renders when the signal changes (zoneless)', async () => {
    const backend = createFakeBackend(['label']);
    appRef = await bootstrapNativeAngular(AnRootComponent, backend, {
      document: backend.document,
    });
    await flush(appRef);

    const root = backend.factory.hosts.find((host) => host !== null)!;
    appRef.components[0]!.instance.label.set('updated');
    await flush(appRef);

    expect(textOf(queryAll(root, 'an-view')[0]!)).toBe('updated');
  });

  it('renders with nothing declared — proof that no property write ever happens', async () => {
    // The strictest possible form of the JIT trap: `label` is NOT declared, so a
    // single `setProperty` call would throw. The render only succeeds if the AOT
    // compiler registered the input and the renderer never saw a property write.
    const backend = createFakeBackend([]);
    appRef = await bootstrapNativeAngular(AnRootComponent, backend, {
      document: backend.document,
    });
    await flush(appRef);

    const root = backend.factory.hosts.find((host) => host !== null)!;
    const view = queryAll(root, 'an-view')[0]!;

    expect(textOf(view)).toBe('hello');
    expect(Object.keys(view.properties)).toEqual([]);
  });

  it('destroys the platform with the application', () => {
    // The teardown helper is the only thing that keeps one test from leaking
    // into the next: Angular tears down the previous platform on the next
    // bootstrap anyway, so a helper that stopped destroying the platform would
    // leave the suite green. This assertion is what makes that visible.
    const backend = createFakeBackend(['label']);
    const app = new Promise<ApplicationRef>((resolve, reject) => {
      bootstrapNativeAngular(AnRootComponent, backend, { document: backend.document }).then(
        resolve,
        reject,
      );
    });

    return app.then((appRef) => {
      const platformRef = appRef.injector.get(PlatformRef);
      disposeNativeApplication(appRef);

      expect(appRef.destroyed).toBe(true);
      expect(platformRef.destroyed).toBe(true);
    });
  });

  it('disposes an application that Angular already tore down', async () => {
    // A second bootstrap destroys the previous platform, and with it the
    // application that owned it — the helper must not throw when it is handed
    // an application whose injector is already gone (`NG0205`).
    const first = createFakeBackend(['label']);
    const firstApp = await bootstrapNativeAngular(AnRootComponent, first, {
      document: first.document,
    });

    const second = createFakeBackend(['label']);
    await bootstrapNativeAngular(AnRootComponent, second, {
      document: second.document,
    });

    expect(firstApp.destroyed).toBe(true);
    expect(() => disposeNativeApplication(firstApp)).not.toThrow();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm vitest run packages/testing/src/aot.e2e.spec.ts`
Expected: FAIL — the spec imports `disposeNativeApplication`, which does not exist yet, and the fixture artefact is missing.

- [ ] **Step 3: Write the teardown helper**

`packages/testing/src/teardown.ts`:

```ts
import { PlatformRef } from '@angular/core';
import type { ApplicationRef, Type } from '@angular/core';

/**
 * Destroys an application and the platform that owns it.
 *
 * Angular allows exactly one platform per realm, and `bootstrapNativeAngular`
 * destroys the previous platform when it bootstraps again — which also destroys
 * the application that owned it, and with it its injector. Reading from a
 * destroyed injector throws `RuntimeError(-205)`, so the `destroyed` check has
 * to come first: the helper must dispose cleanly whether it is called on a
 * live application or on one Angular already tore down.
 */
export function disposeNativeApplication(appRef: ApplicationRef): void {
  if (appRef.destroyed) {
    return;
  }

  const platformRef: PlatformRef | null = appRef.injector.get(PlatformRef, null);

  appRef.destroy();

  if (platformRef && !platformRef.destroyed) {
    platformRef.destroy();
  }
}
```

Add to `packages/testing/src/index.ts`:

```ts
export * from './teardown';
```

- [ ] **Step 4: Write the fixture component**

`packages/testing/fixtures/src/app.ts`:

```ts
import { Component, input, signal } from '@angular/core';

@Component({
  selector: 'an-view',
  standalone: true,
  template: `<span class="label">{{ label() }}</span>`,
})
export class AnViewComponent {
  readonly label = input('');
}

@Component({
  selector: 'an-root',
  standalone: true,
  imports: [AnViewComponent],
  template: `<an-view [label]="label()" class="root" />`,
})
export class AnRootComponent {
  readonly label = signal('hello');
}
```

`packages/testing/fixtures/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM"],
    "experimentalDecorators": true,
    "useDefineForClassFields": false,
    "skipLibCheck": true,
    "strict": true,
    "outDir": "./out",
    "rootDir": "./src",
    "types": []
  },
  "angularCompilerOptions": {
    "strictTemplates": true,
    "compilationMode": "full"
  },
  "files": ["src/app.ts"]
}
```

> `"module": "preserve"` with `"moduleResolution": "bundler"` is required: TypeScript 6 rejects `moduleResolution: "node10"` with TS5107. The fixture is compiled by `ngc` on its own — it is deliberately *outside* the `tsc -b` solution, so the root build never compiles it twice.

- [ ] **Step 5: Write the fixture compilation script**

`scripts/compile-fixtures.mjs`:

```js
import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const project = resolve(root, 'packages/testing/fixtures/tsconfig.json');
const ngc = resolve(root, 'node_modules/@angular/compiler-cli/bundles/src/bin/ngc.js');

// The output directory is emptied first: a renamed fixture would otherwise
// leave a stale artefact behind, and the suite would silently test the old
// component.
rmSync(resolve(root, 'packages/testing/fixtures/out'), { recursive: true, force: true });

const result = spawnSync(process.execPath, [ngc, '-p', project], {
  cwd: root,
  stdio: 'inherit',
});

if (result.error) {
  console.error(`AOT fixture compilation could not start: ${result.error.message}`);
  process.exit(1);
}

if (result.status !== 0) {
  console.error('AOT fixture compilation failed.');
  process.exit(result.status ?? 1);
}

console.log('AOT fixtures compiled to packages/testing/fixtures/out');
```

- [ ] **Step 6: Compile the fixtures and inspect the output**

Run: `pnpm run compile:fixtures`
Expected: prints `AOT fixtures compiled to packages/testing/fixtures/out`.

Run: `grep -o 'inputs: {[^}]*}' packages/testing/fixtures/out/app.js`
Expected: `inputs: { label: [1, "label"] }` — flag `1` is `InputFlags.SignalBased`. **If this is missing, the signal input was not compiled and the end-to-end test must not be trusted.**

Also confirm the static attribute made it into the `consts` table — `ngc` encodes it
as an `AttributeMarker.Classes` entry, never as the literal string `class="root"`:

Run: `grep -o 'consts: \[[^]]*\]' packages/testing/fixtures/out/app.js`
Expected: `consts: [[1, "root", 3, "label"]]` — marker `1` is `AttributeMarker.Classes` and `3` is `AttributeMarker.Bindings`.

- [ ] **Step 7: Run the end-to-end test to verify it passes**

Run: `pnpm vitest run packages/testing/src/aot.e2e.spec.ts`
Expected: 3 tests PASS.

> **One Angular instance.** The fixture is compiled outside the workspace packages and imports `@angular/core` from its own `node_modules`. pnpm points every package at the same store path, so the platform and the fixture must share one Angular instance — if they did not, the platform would reject the fixture's component definition and the render would fail loudly. If a duplicate instance ever appears, add `resolve: { dedupe: ['@angular/core'] }` to `vitest.config.ts`.

- [ ] **Step 8: Verify the negative case — a JIT component must not silently pass**

The third test in Step 1 is that negative case: it renders with **no declared properties**, so the strict `setProperty` rule from Task 4 would throw on the first property write. The render only succeeds because AOT registered the input as a directive input. With a permissive renderer, a JIT-compiled component (where signal inputs are *not* registered) would render an empty label and the suite would still be green. Do not relax the rule.

- [ ] **Step 9: Wire the compilation into the suite**

`package.json` — the suite must never run against a stale artefact:

```json
"test": "pnpm run compile:fixtures && vitest run"
```

Run: `pnpm test`
Expected: the whole suite green, with the fixture compilation running first.

- [ ] **Step 10: Commit**

```bash
git add packages/testing/fixtures scripts/compile-fixtures.mjs packages/testing/src/aot.e2e.spec.ts packages/testing/src/teardown.ts packages/testing/src/index.ts package.json
git commit -m "test: AOT fixture pipeline and end-to-end proof on the fake backend"
```

---

### Task 6: CI, governance and the M0 gap analysis

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `docs/architecture/m0-gap-analysis.md`
- Create: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`
- Create: `packages/core/README.md`, `packages/platform/README.md`, `packages/testing/README.md`
- Modify: `package.json` (add `compile:fixtures` to the `test` script so CI cannot skip it)

**Interfaces:**
- Consumes: everything from Tasks 1–5.
- Produces:
  - `pnpm test` → runs `compile:fixtures` **before** the unit tests.
  - `.github/workflows/ci.yml` → three jobs: `toolchain`, `unit`, `typecheck`.
  - `docs/architecture/m0-gap-analysis.md` → the M0 deliverable record.

- [ ] **Step 1: Write the CI workflow**

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:

jobs:
  toolchain:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 10.34.9
      - uses: actions/setup-node@v4
        with:
          node-version: '22.22.3'
          cache: 'pnpm'
      - run: pnpm install --frozen-lockfile
      - run: pnpm run check:toolchain

  unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 10.34.9
      - uses: actions/setup-node@v4
        with:
          node-version: '22.22.3'
          cache: 'pnpm'
      - run: pnpm install --frozen-lockfile
      - run: pnpm test

  typecheck:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 10.34.9
      - uses: actions/setup-node@v4
        with:
          node-version: '22.22.3'
          cache: 'pnpm'
      - run: pnpm install --frozen-lockfile
      - run: pnpm run build
```

> `pnpm/action-setup` runs **before** `actions/setup-node`: the Node action's
> `cache: 'pnpm'` needs pnpm on the PATH. The three jobs are deliberately
> disjoint — the toolchain job checks the pinned versions, the unit job runs the
> suite (which compiles the AOT fixtures first), and the typecheck job builds
> the TypeScript project references. The branch list carries `main` and
> `master` because the repository's default branch is still `master`.

- [ ] **Step 2: Wire fixture compilation into the test script**

In `package.json`, change:

```json
"test": "pnpm run compile:fixtures && vitest run"
```

- [ ] **Step 3: Write the gap analysis**

`docs/architecture/m0-gap-analysis.md`:

```markdown
# M0 gap analysis — can Angular render to native views?

**Date:** 2026-11-11 · **Status:** resolved — the SPI path works.

## Question

Can an Angular 22 application render into a non-DOM tree through Angular's
public extension points, so that a native backend can take over?

## Answer

**Yes.** Angular 22.2.5 was bootstrapped in Node with no DOM present, using a
custom `Renderer2`, and rendered a real AOT-compiled component tree. The
required extension points are `Renderer2` / `RendererFactory2` (public API) plus
seven `ɵ` internals, all isolated in `packages/platform/src/angular-internals.ts`.

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

## The silent failure that shaped the design

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
4. **Animations.** Angular's animation engine expects DOM-ish APIs; M1 ships
   without `@angular/animations` and drives native animations directly.

## Version floors

- Angular `22.2.x`, TypeScript `6.0.x` — pinned to `22.2.5` and `6.0.3`.
  `@angular/compiler-cli`'s TypeScript peer range is `>=6.0 <6.1`, but it is an
  optional peer, so an incompatible TypeScript is not rejected by the installer;
  `pnpm run check:toolchain` fails on a mismatch.
- Node `>=22.12` (hard), `22.22.3+` recommended — Angular 22's engine floor.
```

- [ ] **Step 4: Write the governance files**

`CONTRIBUTING.md` — how to build, the test commands, the AOT requirement, and the rule that Angular private API is confined to one file.

`packages/{core,platform,testing}/README.md` — one short README per published
package (what it is, how it is used, the version floors). Every README carries
the trademark notice:

> NativeAngular is an independent community project, not affiliated with or endorsed by Google LLC, the Angular team, Expo or Meta. "Angular" is a trademark of Google LLC and is used here only to describe compatibility.

`CODE_OF_CONDUCT.md` — Contributor Covenant 2.1.

`SECURITY.md` — how to report a vulnerability privately; no public issue for exploits.

`CHANGELOG.md`:

```markdown
# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- M0 spike: a DOM-less Angular platform, the backend SPI, and a fake backend
  that runs AOT-compiled components in Node.
```

- [ ] **Step 5: Run the whole suite**

Run: `pnpm run build && pnpm test`
Expected: `tsc -b` succeeds; fixture compilation succeeds; all specs PASS.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml docs/architecture/m0-gap-analysis.md \
  CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md CHANGELOG.md package.json
git commit -m "ci: workflow, governance files and the M0 gap analysis"
```

---

## Definition of done for M0

- [ ] `pnpm install && pnpm run build && pnpm test` is green from a clean clone.
- [ ] An AOT-compiled Angular component renders through the custom renderer into a DOM-less tree in Node.
- [ ] A signal input binds as a directive input, and the test proves no unknown property write reaches the renderer.
- [ ] A signal change re-renders zonelessly.
- [ ] The Angular private API is confined to `packages/platform/src/angular-internals.ts`, and the file documents every import.
- [ ] The renderer contract findings F4–F10 are recorded in `docs/architecture/m0-gap-analysis.md` and each has a test.
- [ ] CI runs toolchain check, typecheck and tests on Node 22.22.x.
- [ ] The repo is licensed MIT, carries the trademark notice and third-party notices.

## Explicitly out of scope for M0

- Fabric backend, device builds, simulators, Rust crates (M1 and M3).
- The capability resolver (M2) — M0 ships capability *declarations* only.
- Style compilation (M1).
- Router integration (M1).
