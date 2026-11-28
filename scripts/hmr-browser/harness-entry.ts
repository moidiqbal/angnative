/**
 * harness-entry.ts — in-page half of the browser HMR evidence harness.
 *
 * This is test wiring, not shipped product code: it imports the REAL dev
 * bundle once, registers it with a REAL HmrRuntime, then calls the REAL
 * startWebHmr with no `subscribe`, `loadModule` or `fetchImpl` override. The
 * only additions are a wrapper around the real client's own EventSource for
 * SSE receive timestamps and a DOM element that makes the applied state
 * observable to Playwright. There is no second observer connection.
 */
import { HmrRuntime, type HmrUpdate } from '@angnative/contract/hmr';
import { startWebHmr } from '../../packages/web/src/hmr';

interface HarnessState {
  ready: boolean;
  error: string | null;
  realmToken: string;
  navigationCount: number;
  startedAt: number;
  sse: Array<{ version: number; url: string; at: number }>;
  applied: Array<{ version: number; text: string; at: number }>;
}

type LiveInstance = {
  variant(): string;
  count(): number;
  setCount(value: number): void;
  snapshot(): { count: number };
  restore(state: unknown): void;
};

declare global {
  interface Window {
    __hmrState: HarnessState;
    __hmrSetCount: (value: number) => void;
    __hmrProbe: () => {
      realmToken: string;
      navigationCount: number;
      generation: number;
      state: unknown;
      count: number;
      variant: string;
      dom: string;
    };
  }
}

const state: HarnessState = {
  ready: false,
  error: null,
  realmToken: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
  navigationCount: performance.getEntriesByType('navigation').length,
  startedAt: Date.now(),
  sse: [],
  applied: [],
};
window.__hmrState = state;

const target = document.getElementById('state')!;

function render(instance: LiveInstance): void {
  target.textContent = `${instance.variant()}:${instance.count()}`;
}

async function boot(): Promise<void> {
  // The real dev server serves this ESM module with a real ngc compile behind
  // it. Import it exactly once; all later versions arrive through HMR blobs.
  // Kept in a variable so esbuild cannot resolve the dev-server URL at build
  // time; the browser resolves it against the DevServer origin at runtime.
  const initialUrl = '/dev-bundle.js';
  const initial = (await import(/* @vite-ignore */ initialUrl)) as {
    hmrModule?: {
      factory: () => LiveInstance;
      hooks: {
        snapshot: (instance: LiveInstance) => unknown;
        restore: (instance: LiveInstance, state: unknown) => void;
        dispose: (instance: LiveInstance) => void;
      };
    };
  };
  const payload = initial.hmrModule;
  if (!payload || typeof payload.factory !== 'function') {
    throw new Error('dev bundle did not export hmrModule.factory');
  }

  const runtime = new HmrRuntime<LiveInstance>();
  runtime.register('root', payload.factory, payload.hooks);
  render(runtime.instance('root')!);

  window.__hmrSetCount = (value: number) => {
    const instance = runtime.instance('root')!;
    instance.setCount(value);
    render(instance);
  };

  window.__hmrProbe = () => {
    const instance = runtime.instance('root')!;
    return {
      realmToken: state.realmToken,
      navigationCount: performance.getEntriesByType('navigation').length,
      generation: runtime.generation('root'),
      state: runtime.state('root'),
      count: instance.count(),
      variant: instance.variant(),
      dom: target.textContent ?? '',
    };
  };

  // Probe-only instrumentation: wrap the global EventSource so the REAL client
  // connection below records its own receive timestamp. No second observer.
  const NativeEventSource = window.EventSource;
  class ObservedEventSource extends NativeEventSource {
    constructor(url: string | URL, config?: EventSourceInit) {
      super(url, config);
      this.addEventListener('hmr', (event: Event) => {
        const update = JSON.parse((event as MessageEvent<string>).data) as HmrUpdate;
        state.sse.push({ version: update.version, url: update.bundles.web, at: Date.now() });
      });
    }
  }
  window.EventSource = ObservedEventSource as unknown as typeof EventSource;

  const unsubscribe = startWebHmr<LiveInstance>({
    runtime,
    moduleId: 'root',
    onApplied: (instance, update) => {
      render(instance);
      state.applied.push({ version: update.version, text: target.textContent!, at: Date.now() });
    },
  });
  window.addEventListener('beforeunload', unsubscribe);

  state.ready = true;
}

void boot().catch((error: unknown) => {
  state.error = error instanceof Error ? `${error.message}\n${error.stack ?? ''}` : String(error);
});
