/**
 * main.router.ts — AOT demo for full Angular Router navigation in QuickJS.
 *
 * Bootstraps a standalone app with two routes and a <router-outlet>, using the
 * native renderer and zoneless change detection. Exposes __navigate on
 * globalThis so the Rust acceptance test can attempt navigation.
 *
 * STATUS (honest): bootstrap and outlet anchor work. Navigation is requested
 * (hasRequestedNavigation=true, 2 routes configured) but Angular Router never
 * emits navigation events in this minimal QuickJS event loop; route-component
 * swapping is therefore not verified. See docs/UNSUPPORTED-FEATURES.md.
 */
import { Component, provideZonelessChangeDetection, type ApplicationRef } from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import { provideRouter, Router, RouterOutlet, type Routes } from '@angular/router';
import { provideNativeAngular } from '@angnative/platform';
import {
  NaViewComponent as NaView,
  NaTextComponent as NaText,
} from '@angnative/common';

@Component({
  selector: 'demo-home',
  standalone: true,
  imports: [NaView, NaText],
  template: `
    <na-view>
      <na-text>HOME ROUTE</na-text>
    </na-view>
  `,
})
export class HomeComponent {}

@Component({
  selector: 'demo-about',
  standalone: true,
  imports: [NaView, NaText],
  template: `
    <na-view>
      <na-text>ABOUT ROUTE</na-text>
    </na-view>
  `,
})
export class AboutComponent {}

@Component({
  selector: 'demo-router-app',
  standalone: true,
  imports: [RouterOutlet],
  template: `<router-outlet></router-outlet>`,
})
export class RouterAppComponent {}

const routes: Routes = [
  { path: '', component: HomeComponent },
  { path: 'about', component: AboutComponent },
];

declare global {
  // eslint-disable-next-line no-var
  var __navigate: ((url: string) => Promise<void>) | undefined;
  // eslint-disable-next-line no-var
  var __routerError: string | undefined;
}

bootstrapApplication(RouterAppComponent, {
  providers: [
    provideZonelessChangeDetection(),
    provideNativeAngular(),
    provideRouter(routes),
  ],
})
  .then((ref: ApplicationRef) => {
    // APP_BOOTSTRAP_LISTENER does not fire in this runtime; request explicitly.
    ref.injector.get(Router).initialNavigation();
    globalThis.__navigate = (url: string) =>
      ref.injector.get(Router).navigateByUrl(url).then(() => undefined);
  })
  .catch((err: unknown) => {
    globalThis.__routerError = String((err as { message?: string })?.message ?? err);
  });
