# @angnative/web

DOM fallback adapter for AngNative. It runs the same Angular components and
the same mutation protocol as the native shells, but applies the resulting
`PlatformOp`s to the browser DOM instead of native widgets.

## Status

**Available.** Created in Phase 5, with 6 passing tests
(`pnpm --filter @angnative/web test`) and a working AOT demo (`pnpm start`).

The browser demo no longer loads `@angular/compiler`. It uses real `ngc`
Ivy output from `tsconfig.aot.web.json`, bundled by
`scripts/bundle-web-aot.mjs`. The build fails if a JIT compiler package enters
the bundle metafile.

## What it provides

- `DomBackend` — applies `Create / SetProps / SetText / Append / Insert /
  Remove / Destroy / SetFrame` operations to a `Document`, keeping a
  node-id → DOM-node map.
- `WebRenderer` / `WebRendererFactory2` — a `Renderer2` that encodes the same
  `CommandBatch` as `@angnative/core`, with DOM-specific host adoption and
  listener forwarding.
- `bootstrapWebApp(rootComponent, options?)` — `bootstrapApplication` wrapper,
  zoneless by default.
- Re-exports of the `na-*` primitives from `@angnative/common`.

## Usage

The demo entry is AOT-compiled; it contains no JIT compiler import:

```ts
import { bootstrapWebApp } from '@angnative/web';
import { AppComponent } from './app.component';

bootstrapWebApp(AppComponent);
```

From the repository root:

```bash
pnpm start
# equivalent sequence:
pnpm --filter @angnative/web run build
pnpm compile:aot:web
pnpm bundle:aot:web
node scripts/serve-web.mjs
```

The server reads `dist/web-demo/app.js` and serves the demo at
`http://localhost:4173`. The single critical module script is discovered in
`<head>` and marked `fetchpriority="high"`; there are no parser-blocking or
non-critical scripts.

## Tests

```bash
pnpm --filter @angnative/web test
pnpm test   # root vitest picks up packages/web/src/*.spec.ts
```

The integration spec uses a checked-in, real `ngc` fixture at
`packages/testing/fixtures/web-aot/compiled/`. Regenerate it after editing the
fixture source with:

```bash
pnpm compile:aot:web-fixture
```
