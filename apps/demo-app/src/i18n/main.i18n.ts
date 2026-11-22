/**
 * main.i18n.ts — AOT demo for i18n markers in a real Angular template.
 *
 * The `i18n` marker makes ngc emit a localizable message in the template
 * function. The Rust acceptance test registers a translation with
 * `$localize.loadTranslations()` before this bundle is evaluated.
 */
import {
  Component,
  provideZonelessChangeDetection,
  type ApplicationRef,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import { provideNativeAngular } from '@angnative/platform';
import {
  NaTextComponent as NaText,
  NaViewComponent as NaView,
} from '@angnative/common';

@Component({
  selector: 'demo-i18n-app',
  standalone: true,
  imports: [NaView, NaText],
  template: `
    <na-view>
      <na-text i18n>Hello {{ name }}!</na-text>
    </na-view>
  `,
})
export class I18nAppComponent {
  name = 'AngNative';
}

declare global {
  // eslint-disable-next-line no-var
  var __i18nError: string | undefined;
}

bootstrapApplication(I18nAppComponent, {
  providers: [provideZonelessChangeDetection(), provideNativeAngular()],
}).catch((err: unknown) => {
  const error = err as { message?: string; stack?: string };
  globalThis.__i18nError = String(error.stack ?? error.message ?? err);
});
