/**
 * bootstrap.ts — `bootstrapNativeAngularApp()`.
 *
 * Bootstraps a standalone Angular component on the AngNative native renderer
 * (no DOM, no BrowserModule, zoneless).
 *
 * Example:
 *   import { bootstrapApplication } from '@angular/platform-browser';
 *   import { bootstrapNativeAngularApp } from '@angnative/platform';
 *
 *   bootstrapNativeAngularApp(AppComponent, {
 *     providers: [],
 *   });
 */

import {
  provideZoneChangeDetection,
  type EnvironmentProviders,
  type ApplicationRef,
  type Provider,
  type Type,
} from '@angular/core';
import { provideNativeAngular, type NativeAngularConfig } from './native-angular';

/** Options for `bootstrapNativeAngularApp`. */
export interface NativeAngularBootstrapOptions {
  /** Additional Angular providers (services, etc.). */
  providers?: Provider[];
  /** Optional native AngNative config (flush handler, etc.). */
  nativeConfig?: NativeAngularConfig;
  /** Enable zoneless change detection (default: true). */
  zoneless?: boolean;
}

/**
 * Bootstrap a standalone Angular component on the AngNative native renderer.
 *
 * Replaces `platformBrowserDynamic().bootstrapModule(AppModule)` / DOM bootstrapping.
 * The app runs with no WebView — Angular's change detection drives the native
 * view tree through `AngNativeRenderer2`, and mutations are flushed to the
 * Rust host (or fake backend) via the command protocol.
 */
export async function bootstrapNativeAngularApp(
  rootComponent: Type<any>,
  options: NativeAngularBootstrapOptions = {},
): Promise<ApplicationRef> {
  const appProviders: Array<Provider | EnvironmentProviders> = [];

  if (options.zoneless !== false) {
    appProviders.push(provideZoneChangeDetection({ eventCoalescing: true }));
  }

  appProviders.push(...provideNativeAngular(options.nativeConfig));
  appProviders.push(...(options.providers ?? []));

  // Load the browser bootstrap bridge only when an app actually boots. This
  // keeps importing native providers and test helpers free of browser runtime
  // initialization; the host app supplies Angular's linker in its build.
  const { bootstrapApplication } = await import('@angular/platform-browser');
  return bootstrapApplication(rootComponent, {
    providers: appProviders,
  });
}
