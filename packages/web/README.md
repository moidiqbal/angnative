# @angnative/web

DOM fallback adapter for AngNative. It runs the same Angular components and
the same mutation protocol as the native shells, but applies the resulting
`PlatformOp`s to the browser DOM instead of native widgets.

## Status

**Available.** Created in Phase 5, with 6 passing tests
(`pnpm --filter @angnative/web test`) and a working demo (`pnpm start`).

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

```ts
import '@angular/compiler'; // JIT until the ngc pipeline is wired up
import { bootstrapWebApp } from '@angnative/web';
import { AppComponent } from './app.component';

bootstrapWebApp(AppComponent);
```

`pnpm start` from the repository root builds this package and serves
`apps/demo-app` at `http://localhost:4173`.

## Tests

```bash
pnpm --filter @angnative/web test
pnpm test   # root vitest picks up packages/web/src/*.spec.ts
```
