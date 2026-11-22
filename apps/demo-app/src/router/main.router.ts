/**
 * main.router.ts — AOT demo for full Angular Router navigation in QuickJS.
 *
 * Bootstraps a standalone app with two routes and a <router-outlet>, using the
 * native renderer and zoneless change detection. Exposes __navigate on
 * globalThis so the Rust acceptance test can perform navigation.
 *
 * STATUS: FULL ROUTE NAVIGATION VERIFIED ✅
 * Bootstrap, <router-outlet> anchor, initial navigation ('/') and subsequent
 * imperative navigation ('/about') work end-to-end with native view/text swapping.
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
  var __navigate: ((url: string) => Promise<boolean>) | undefined;
  // eslint-disable-next-line no-var
  var __routerError: string | undefined;
  // eslint-disable-next-line no-var
  var __router: Router | undefined;
  // eslint-disable-next-line no-var
  var __routerEvents: string[] | undefined;
}

bootstrapApplication(RouterAppComponent, {
  providers: [
    provideZonelessChangeDetection(),
    provideNativeAngular(),
    provideRouter(routes),
  ],
})
  .then((ref: ApplicationRef) => {
    const router = ref.injector.get(Router);
    globalThis.__router = router;
    const events: string[] = [];
    globalThis.__routerEvents = events;
    router.events.subscribe((e: any) => {
      events.push(e.constructor.name);
    });
    router.initialNavigation();
    globalThis.__navigate = (url: string) => router.navigateByUrl(url);
  })
  .catch((err: unknown) => {
    globalThis.__routerError = String((err as { message?: string })?.message ?? err);
  });
