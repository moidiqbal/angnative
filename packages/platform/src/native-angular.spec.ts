import { describe, expect, it, beforeEach, vi } from 'vitest';
import { ErrorHandler, Renderer2, RendererFactory2 } from '@angular/core';
import { AngNativeRendererFactory2, ElementRegistry } from '@angnative/core';
import {
  createDefaultFlushHandler,
  NativeErrorHandler,
  provideNativeErrorHandler,
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
    expect(tokens).toContain(ErrorHandler);
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

describe('native ErrorHandler provider', () => {
  beforeEach(() => {
    globalThis.__angnativeOnError = undefined;
  });

  function errorHandlerFrom(
    providers: Array<{ provide: unknown; useValue?: unknown }>,
  ): NativeErrorHandler {
    const provider = providers.find((p) => p.provide === ErrorHandler);
    expect(provider?.useValue).toBeInstanceOf(NativeErrorHandler);
    return provider!.useValue as NativeErrorHandler;
  }

  it('provides ErrorHandler by default and allows opting out', () => {
    const enabled = errorHandlerFrom(provideNativeRendererFactory() as any);
    expect(enabled).toBeInstanceOf(NativeErrorHandler);

    const tokens = (
      provideNativeRendererFactory({ errorHandler: false }) as Array<{ provide: unknown }>
    ).map((p) => p.provide);
    expect(tokens).not.toContain(ErrorHandler);
    expect(provideNativeErrorHandler({ errorHandler: false })).toEqual([]);
  });

  it('forwards the error and stack to the host-installed hook', () => {
    const hook = vi.fn();
    globalThis.__angnativeOnError = hook;
    const handler = errorHandlerFrom(provideNativeErrorHandler() as any);

    const error = new Error('boom');
    handler.handleError(error);

    expect(hook).toHaveBeenCalledTimes(1);
    expect(hook).toHaveBeenCalledWith(error, error.stack);
  });

  it('uses a configured custom hook instead of the global one', () => {
    const custom = vi.fn();
    const globalHook = vi.fn();
    globalThis.__angnativeOnError = globalHook;
    const handler = errorHandlerFrom(
      provideNativeErrorHandler({ errorHandler: custom }) as any,
    );

    const error = new Error('custom boom');
    handler.handleError(error);

    expect(custom).toHaveBeenCalledWith(error, error.stack);
    expect(globalHook).not.toHaveBeenCalled();
  });
});
