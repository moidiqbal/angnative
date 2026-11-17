/**
 * main.ts — demo app entry point.
 *
 * Bootstraps the Angular app on the AngNative native renderer via
 * @angnative/platform. Zoneless, no BrowserModule, no WebView.
 */

import { bootstrapNativeAngularApp } from '@angnative/platform';
import { AppComponent } from './app.component';

bootstrapNativeAngularApp(AppComponent, {
  providers: [],
  zoneless: true,
}).then((ref) => {
  // eslint-disable-next-line no-console
  console.log('[angnative demo] app bootstrapped on native renderer');
}).catch((err) => {
  // eslint-disable-next-line no-console
  console.error('[angnative demo] bootstrap failed:', err);
});
