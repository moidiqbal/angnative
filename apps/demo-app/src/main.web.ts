/**
 * main.web.ts — browser entry for the demo app using @angnative/web.
 *
 * The same Angular components render through a DOM-backed PlatformOp adapter.
 * This entry is compiled with ngc via tsconfig.aot.web.json, so the browser
 * receives Ivy component definitions and never loads @angular/compiler.
 */
import { bootstrapWebApp } from '@angnative/web';
import { AppComponent } from './app.component';

bootstrapWebApp(AppComponent).catch((err) => {
  // eslint-disable-next-line no-console
  console.error('[angnative demo] web bootstrap failed:', err);
});
