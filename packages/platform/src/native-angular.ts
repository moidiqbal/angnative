/**
 * native-angular.ts — `provideNativeAngular()` and config types.
 *
 * Angular providers that swap the DOM renderer for AngNative's custom
 * `AngNativeRendererFactory2`. Use these instead of BrowserModule when
 * bootstrapping an Angular app for native (Android / iOS / Desktop / Web)
 * rendering.
 *
 * Zoneless: `bootstrapNativeAngularApp` uses Angular's
 * `provideZonelessChangeDetection()` by default (no zone.js). Set
 * `zoneless: false` to opt into `provideZoneChangeDetection`.
 */

import {
  type Provider,
  ErrorHandler,
  RendererFactory2,
  Renderer2,
} from '@angular/core';
import {
  AngNativeRendererFactory2,
  AngNativeRenderer,
  ElementRegistry,
} from '@angnative/core';
import { type FlushHandler } from '@angnative/core/renderer';

/** Host error hook installed by the native shell (P1-09). */
export type HostErrorHook = (error: unknown, stack?: string) => void;

/** Enable/disable native error forwarding, or supply a custom host hook. */
export type NativeErrorHandlerConfig = boolean | HostErrorHook;

/** Configuration for the native Angular platform. */
export interface NativeAngularConfig {
  /** Flush handler that delivers command batches to the backend (Rust host or fake backend). */
  flush?: FlushHandler;
  /**
   * Error forwarding to the host. Defaults to `true`: installs
   * `NativeErrorHandler`, which forwards to `globalThis.__angnativeOnError`.
   * Pass `false` to keep Angular's default handler, or a function to use as a
   * custom host hook.
   */
  errorHandler?: NativeErrorHandlerConfig;
}

/** Global hook native shells can install to receive encoded batches. */
type HostDispatch = (batchJson: string) => void;

declare global {
  // eslint-disable-next-line no-var
  var __angnativeBatches: string[] | undefined;
  // eslint-disable-next-line no-var
  var __angnativeDispatchBatch: HostDispatch | undefined;
  /** Installed by the Rust runtime bridge; forwards to `na_reportError`. */
  // eslint-disable-next-line no-var
  var __angnativeOnError: HostErrorHook | undefined;
}

/**
 * Default flush handler. Keeps batches observable on `globalThis` for dev
 * tooling and calls a host-installed `__angnativeDispatchBatch` when present.
 * Platform shells override this by passing `nativeConfig.flush`.
 */
export function createDefaultFlushHandler(): FlushHandler {
  return {
    onFlush(batchJson: string): void {
      globalThis.__angnativeBatches ??= [];
      globalThis.__angnativeBatches.push(batchJson);
      globalThis.__angnativeDispatchBatch?.(batchJson);
    },
  };
}

/**
 * Angular `ErrorHandler` that reports uncaught app errors to the native host.
 * A custom hook (from `NativeAngularConfig.errorHandler`) takes precedence;
 * otherwise the host-installed `__angnativeOnError` global is used. When no
 * hook is installed, Angular's default logging still applies.
 */
export class NativeErrorHandler extends ErrorHandler {
  constructor(private readonly hostHook?: HostErrorHook) {
    super();
  }

  override handleError(error: any): void {
    const hook = this.hostHook ?? globalThis.__angnativeOnError;
    const stack = error instanceof Error ? error.stack : undefined;
    if (typeof hook === 'function') {
      try {
        hook(error, stack);
        return;
      } catch (hookError) {
        // A broken host hook must never mask the application error.
        console.error('__angnativeOnError failed', hookError);
      }
    }
    super.handleError(error);
  }
}

/** Build the optional native `ErrorHandler` provider. */
export function provideNativeErrorHandler(config: NativeAngularConfig = {}): Provider[] {
  if (config.errorHandler === false) return [];
  const hook = typeof config.errorHandler === 'function' ? config.errorHandler : undefined;
  return [{ provide: ErrorHandler, useValue: new NativeErrorHandler(hook) }];
}

/**
 * Create the AngNative `RendererFactory2` provider. The factory is a singleton
 * per application; `createRenderer` returns the shared renderer instance.
 */
export function provideNativeRendererFactory(config?: NativeAngularConfig): Provider[] {
  const factory = new AngNativeRendererFactory2({
    flush: config?.flush ?? createDefaultFlushHandler(),
  });

  return [
    // The factory token must be provided explicitly: primitives inject it, and
    // `AngNativeRendererFactory2` is intentionally not @Injectable.
    { provide: AngNativeRendererFactory2, useValue: factory },
    { provide: RendererFactory2, useValue: factory },
    {
      provide: Renderer2,
      useFactory: (rendererFactory: RendererFactory2) =>
        rendererFactory.createRenderer(null, null),
      deps: [RendererFactory2],
    },
    {
      provide: ElementRegistry,
      useFactory: () => factory.getRenderer().getRegistry(),
    },
    ...provideNativeErrorHandler(config),
  ];
}

/**
 * `provideNativeAngular()` — the single provider function to call in your
 * bootstrap options.
 *
 * Example:
 *   bootstrapApplication(AppComponent, {
 *     providers: [
 *       provideZoneChangeDetection({ eventCoalescing: true }),
 *       provideNativeAngular(),
 *     ],
 *   });
 *
 * To wire a Rust-host flush handler (FFI), pass it via config:
 *   provideNativeAngular({ flush: (batchJson) => rustHost.dispatch(batchJson) })
 */
export function provideNativeAngular(
  config?: NativeAngularConfig,
): Provider[] {
  return provideNativeRendererFactory(config);
}

/** Re-export the renderer factory type for consumers who need direct access. */
export { AngNativeRendererFactory2, AngNativeRenderer };
export type { FlushHandler };



