/**
 * main.web.ts — browser entry for the demo app using @angnative/web.
 * The same Angular components render through a DOM-backed PlatformOp adapter.
 */
import '@angular/compiler'; // JIT compiler — no ngc pipeline is wired up yet.
import { bootstrapWebApp } from '@angnative/web';
import { AppComponent } from './app.component';

bootstrapWebApp(AppComponent).catch((err) => {
  // eslint-disable-next-line no-console
  console.error('[angnative demo] web bootstrap failed:', err);
});
