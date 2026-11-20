/**
 * bootstrap.ts — browser bootstrap wrapper for AngNative apps.
 *
 * Uses Angular's `bootstrapApplication` and installs the web renderer factory.
 * Zoneless change detection is the default (Angular 21+); pass
 * `zoneless: false` when your app loads zone.js and wants
 * `provideZoneChangeDetection({ eventCoalescing: true })`.
 */

import {
  provideZonelessChangeDetection,
  provideZoneChangeDetection,
  type ApplicationRef,
  type Provider,
  type Type,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import { DomBackend } from './dom-backend';
import { provideWebRendererFactory } from './renderer';

export interface BootstrapWebOptions {
  /** Use zone-based change detection instead of the zoneless default. Requires zone.js. */
  zoneless?: boolean;
  /** Document to render into. Defaults to `globalThis.document`. */
  doc?: Document;
  /** Pre-built backend (use this to inspect the produced DOM). */
  backend?: DomBackend;
  /** Additional application providers. */
  providers?: Provider[];
}

/**
 * Bootstrap a standalone Angular component through the AngNative web adapter.
 */
export async function bootstrapWebApp<T>(
  rootComponent: Type<T>,
  options: BootstrapWebOptions = {},
): Promise<ApplicationRef> {
  const doc = options.doc ?? globalThis.document;
  if (!doc) {
    throw new Error('bootstrapWebApp requires a DOM Document');
  }
  const backend = options.backend ?? new DomBackend(doc);
  const changeDetection =
    options.zoneless === false
      ? provideZoneChangeDetection({ eventCoalescing: true })
      : provideZonelessChangeDetection();

  return bootstrapApplication(rootComponent, {
    providers: [
      changeDetection,
      ...provideWebRendererFactory({ doc, backend }),
      ...(options.providers ?? []),
    ],
  });
}
