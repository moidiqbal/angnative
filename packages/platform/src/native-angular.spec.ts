import { describe, expect, it, beforeEach, vi } from 'vitest';
import { Renderer2, RendererFactory2 } from '@angular/core';
import { AngNativeRendererFactory2, ElementRegistry } from '@angnative/core';
import {
  createDefaultFlushHandler,
  provideNativeRendererFactory,
} from './native-angular';

describe('provideNativeRendererFactory', () => {
  it('provides the renderer factory token that primitives inject', () => {
    const providers = provideNativeRendererFactory() as Array<{ provide: unknown }>;
    const tokens = providers.map((p) => p.provide);

    expect(tokens).toContain(AngNativeRendererFactory2);
    expect(tokens).toContain(RendererFactory2);
    expect(tokens).toContain(Renderer2);
    expect(tokens).toContain(ElementRegistry);
  });

  it('uses the single factory instance for all renderer tokens', () => {
    const providers = provideNativeRendererFactory() as Array<{
      provide: unknown;
      useValue?: unknown;
      useFactory?: () => unknown;
    }>;
    const factoryProvider = providers.find(
      (p) => p.provide === AngNativeRendererFactory2,
    );
    const rendererFactoryProvider = providers.find(
      (p) => p.provide === RendererFactory2,
    );

    expect(factoryProvider?.useValue).toBeInstanceOf(AngNativeRendererFactory2);
    expect(rendererFactoryProvider?.useValue).toBe(factoryProvider?.useValue);
  });
});

describe('createDefaultFlushHandler', () => {
  beforeEach(() => {
    globalThis.__angnativeBatches = undefined;
    globalThis.__angnativeDispatchBatch = undefined;
  });

  it('queues batches and invokes a host-installed dispatcher', () => {
    const dispatch = vi.fn();
    globalThis.__angnativeDispatchBatch = dispatch;
    const handler = createDefaultFlushHandler();

    handler.onFlush('{"protocolVersion":1}');

    expect(globalThis.__angnativeBatches).toEqual(['{"protocolVersion":1}']);
    expect(dispatch).toHaveBeenCalledWith('{"protocolVersion":1}');
  });
});
