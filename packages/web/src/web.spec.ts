/**
 * End-to-end web adapter test: a real Angular component bootstrapped through
 * @angnative/web renders into a DOM and receives a click event.
 *
 * happy-dom globals are installed BEFORE Angular modules are imported, so the
 * same spec works both via `pnpm --filter @angnative/web test` and via the
 * repository-root `pnpm test` (which has no vitest setup file).
 */

import { beforeAll, describe, it, expect } from 'vitest';
import { Window } from 'happy-dom';

type Win = InstanceType<typeof Window>;

let win: Win;
let Component: typeof import('@angular/core').Component;
let bootstrapWebApp: typeof import('./bootstrap').bootstrapWebApp;
let DomBackend: typeof import('./dom-backend').DomBackend;
let commandsToPlatformOps: typeof import('./renderer').commandsToPlatformOps;
let NaViewComponent: typeof import('./primitives').NaViewComponent;
let NaTextComponent: typeof import('./primitives').NaTextComponent;

beforeAll(async () => {
  win = new Window({ url: 'http://localhost/' });
  const g = globalThis as unknown as Record<string, unknown>;
  const globals: Record<string, unknown> = {
    window: win,
    document: win.document,
    navigator: win.navigator,
    Node: win.Node,
    Element: win.Element,
    HTMLElement: win.HTMLElement,
    Text: win.Text,
    Comment: win.Comment,
    DocumentFragment: win.DocumentFragment,
    Event: win.Event,
    CustomEvent: win.CustomEvent,
    MouseEvent: win.MouseEvent,
    getComputedStyle: (...args: unknown[]) =>
      (win as unknown as { getComputedStyle: (...a: unknown[]) => unknown }).getComputedStyle(...args),
  };
  for (const [key, value] of Object.entries(globals)) {
    Object.defineProperty(g, key, { value, configurable: true, writable: true });
  }

  // JIT template compilation needs the compiler loaded before components run.
  await import('@angular/compiler');
  const core = await import('@angular/core');
  Component = core.Component;
  const primitives = await import('./primitives');
  NaViewComponent = primitives.NaViewComponent;
  NaTextComponent = primitives.NaTextComponent;
  const backendModule = await import('./dom-backend');
  DomBackend = backendModule.DomBackend;
  const rendererModule = await import('./renderer');
  commandsToPlatformOps = rendererModule.commandsToPlatformOps;
  const bootstrapModule = await import('./bootstrap');
  bootstrapWebApp = bootstrapModule.bootstrapWebApp;
});

describe('@angnative/web integration', () => {
  it('renders an Angular component into the DOM and dispatches a click', async () => {
    const doc = win.document;
    doc.body.innerHTML = '<app-web-test></app-web-test>';

    @Component({
      selector: 'app-web-test',
      standalone: true,
      imports: [NaViewComponent, NaTextComponent],
      template: `
        <na-view [padding]="'8px'">
          <na-text>{{ label }}</na-text>
          <div id="tap" (click)="onClick()">Tap</div>
        </na-view>
      `,
    })
    class TestApp {
      label = 'Hello';
      clicks = 0;
      onClick(): void {
        this.clicks++;
        this.label = `Clicked ${this.clicks}`;
      }
    }

    const backend = new DomBackend(doc);
    const app = await bootstrapWebApp(TestApp, { doc, backend });

    const root = doc.querySelector('app-web-test') as HTMLElement;
    expect(root).not.toBeNull();
    expect(root.querySelector('na-view')).not.toBeNull();
    expect(root.textContent).toContain('Hello');

    const tap = root.querySelector('#tap') as HTMLElement;
    expect(tap).not.toBeNull();
    expect(tap.textContent).toContain('Tap');

    tap.dispatchEvent(new win.Event('click', { bubbles: true }));
    app.tick();

    const instance = app.components[0]!.instance as TestApp;
    expect(instance.clicks).toBe(1);
    expect(root.textContent).toContain('Clicked 1');

    // The renderer factory is reachable through the core DI token, so the
    // re-exported na-* primitives can inject it on the web.
    expect(backend.getNode(2)).toBeDefined();
    app.destroy();
  });

  it('translates core commands into ordered platform ops', () => {
    const ops = commandsToPlatformOps([
      { kind: 'CreateNode', id: 1, nodeType: 'View' },
      { kind: 'CreateNode', id: 2, nodeType: 'Text' },
      { kind: 'SetStyle', id: 1, key: 'padding', value: '8px' },
      { kind: 'SetProperty', id: 1, key: 'attr:id', value: 'root' },
      { kind: 'SetProperty', id: 2, key: 'listener:click:1', value: { eventName: 'click', listenerId: 1 } },
      { kind: 'InsertChild', parentId: 1, childId: 2, index: 0 },
      { kind: 'SetTextContent', id: 2, text: 'hi' },
      { kind: 'RemoveChild', parentId: 1, childId: 2 },
      { kind: 'RemoveNode', id: 2 },
    ]);

    expect(ops.map((op) => op.kind)).toEqual([
      'Create',
      'Create',
      'SetProps',
      'Insert',
      'SetText',
      'Remove',
      'Destroy',
    ]);
    const props = ops.find((op) => op.kind === 'SetProps');
    expect(props).toMatchObject({
      node_id: 1,
      props: { styles: { padding: '8px' }, properties: { 'attr:id': 'root' } },
    });
  });
});
