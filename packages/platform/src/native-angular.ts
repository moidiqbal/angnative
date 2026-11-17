/**
 * native-angular.ts — `provideNativeAngular()` and config types.
 *
 * Angular providers that swap the DOM renderer for AngNative's custom
 * `AngNativeRendererFactory2`. Use these instead of BrowserModule when
 * bootstrapping an Angular app for native (Android / iOS / Desktop / Web)
 * rendering.
 *
 * Zoneless: AngNative is zoneless by design — pair this with
 * `provideZoneChangeDetection({ eventCoalescing: true })` in your app
 * bootstrap options (see `bootstrapNativeAngularApp`).
 */

import {
  type Provider,
  RendererFactory2,
  Renderer2,
  inject,
} from '@angular/core';
import {
  AngNativeRendererFactory2,
  AngNativeRenderer,
  ElementRegistry,
} from '@angnative/core';
import { type FlushHandler } from '@angnative/core/renderer';

/** Configuration for the native Angular platform. */
export interface NativeAngularConfig {
  /** Flush handler that delivers command batches to the backend (Rust host or fake backend). */
  flush?: FlushHandler;
}

/**
 * Create the AngNative `RendererFactory2` provider. The factory is a singleton
 * per application; `createRenderer` returns the shared renderer instance.
 */
export function provideNativeRendererFactory(config?: NativeAngularConfig): Provider[] {
  return [
    {
      provide: RendererFactory2,
      useFactory: () => {
        const factory = new AngNativeRendererFactory2({
          flush: config?.flush ?? undefined,
        });
        return factory;
      },
    },
    {
      provide: Renderer2,
      useFactory: (factory: RendererFactory2) => factory.createRenderer(null, null),
      deps: [RendererFactory2],
    },
    {
      provide: ElementRegistry,
      useFactory: () => {
        const factory = inject(AngNativeRendererFactory2);
        return factory.getRenderer().getRegistry();
      },
    },
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



