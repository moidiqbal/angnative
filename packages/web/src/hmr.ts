/**
 * hmr.ts — browser half of the shared HMR pipeline.
 *
 * The dev server broadcasts a versioned `/hmr` event. This client loads the
 * new bundle as a blob ES module, applies the shared HmrRuntime contract and
 * calls `onApplied` so the host can re-render (for example `appRef.tick()`).
 * No full page reload occurs, so local component state survives.
 */

import {
  applyHmrUpdate,
  isHmrUpdate,
  type HmrModulePayload,
  type HmrRuntime,
  type HmrUpdate,
} from '@angnative/contract/hmr';

export interface WebHmrOptions<T> {
  runtime: HmrRuntime;
  moduleId: string;
  endpoint?: string;
  /** Called after a successful replacement with the live instance. */
  onApplied?: (instance: T, update: HmrUpdate) => void;
  /** Injectable for tests; defaults to global `fetch`. */
  fetchImpl?: typeof fetch;
  /** Injectable for tests / non-browser hosts. */
  subscribe?: (handler: (update: HmrUpdate) => void) => () => void;
  /** Injectable module loader; defaults to blob-module import of the bundle. */
  loadModule?: (update: HmrUpdate) => HmrModulePayload<T> | Promise<HmrModulePayload<T>>;
}

/** Fetch and evaluate a dev bundle as a blob ES module. */
export async function loadWebHmrModule<T>(
  update: HmrUpdate,
  fetchImpl: typeof fetch = fetch,
): Promise<HmrModulePayload<T>> {
  const response = await fetchImpl(update.bundles.web);
  if (!response.ok) throw new Error(`HMR bundle fetch failed: ${response.status}`);
  const source = await response.text();
  const blobUrl = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
  try {
    const module = (await import(/* @vite-ignore */ blobUrl)) as {
      hmrModule?: HmrModulePayload<T>;
      default?: HmrModulePayload<T>;
    };
    const payload = module.hmrModule ?? module.default;
    if (!payload || typeof payload.factory !== 'function') {
      throw new Error('HMR bundle does not export hmrModule');
    }
    return payload;
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

/** Subscribe to the dev server's SSE stream. */
function defaultSubscribe(handler: (update: HmrUpdate) => void): () => void {
  const source = new EventSource('/hmr');
  source.onmessage = (event: MessageEvent<string>) => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(event.data);
    } catch {
      return;
    }
    if (isHmrUpdate(parsed)) handler(parsed);
  };
  return () => source.close();
}

/** Start listening for HMR updates; returns an unsubscribe function. */
export function startWebHmr<T>(options: WebHmrOptions<T>): () => void {
  const last = { value: 0 };
  const load = options.loadModule ?? ((update: HmrUpdate) => loadWebHmrModule<T>(update, options.fetchImpl));
  const handler = (update: HmrUpdate) => {
    if (update.moduleId !== options.moduleId) return;
    void applyHmrUpdate(options.runtime, update, load, last).then((applied) => {
      if (applied) {
        options.onApplied?.(options.runtime.instance<T>(options.moduleId)!, update);
      }
    });
  };
  return options.subscribe ? options.subscribe(handler) : defaultSubscribe(handler);
}
