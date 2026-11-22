/**
 * main.forms.ts — AOT demo for template-driven Angular forms in QuickJS.
 *
 * Uses FormsModule / ngModel on a plain <input>, which the native renderer maps
 * to the `na-text-input` primitive. Exposes small globals so the Rust acceptance
 * test can set the model, flush change detection, and read the resulting model.
 */
import {
  ChangeDetectorRef,
  Component,
  provideZonelessChangeDetection,
  type ApplicationRef,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { bootstrapApplication } from '@angular/platform-browser';
import { provideNativeAngular } from '@angnative/platform';
import {
  NaTextComponent as NaText,
  NaViewComponent as NaView,
} from '@angnative/common';

@Component({
  selector: 'demo-forms-app',
  standalone: true,
  imports: [FormsModule, NaView, NaText],
  template: `
    <na-view>
      <na-text id="model-label">{{ model }}</na-text>
      <input id="name-input" [ngModel]="model" (ngModelChange)="onModelChange($event)" />
    </na-view>
  `,
})
export class FormsAppComponent {
  model = 'initial';
  changes: string[] = [];

  constructor(private readonly cdr: ChangeDetectorRef) {}

  onModelChange(value: string): void {
    this.model = value ?? '';
    this.changes.push(this.model);
  }

  setModel(value: string): void {
    this.model = value ?? '';
    this.flush();
  }

  flush(): void {
    this.cdr.detectChanges();
  }
}

declare global {
  // eslint-disable-next-line no-var
  var __formsApp: FormsAppComponent | undefined;
  // eslint-disable-next-line no-var
  var __formsError: string | undefined;
  // eslint-disable-next-line no-var
  var __formsModel: (() => string) | undefined;
  // eslint-disable-next-line no-var
  var __setFormsModel: ((value: string) => void) | undefined;
  // eslint-disable-next-line no-var
  var __flushForms: (() => void) | undefined;
}

bootstrapApplication(FormsAppComponent, {
  providers: [provideZonelessChangeDetection(), provideNativeAngular()],
})
  .then((ref: ApplicationRef) => {
    const app = ref.components[0].instance as FormsAppComponent;
    globalThis.__formsApp = app;
    globalThis.__formsModel = () => app.model;
    globalThis.__setFormsModel = (value: string) => app.setModel(value);
    globalThis.__flushForms = () => app.flush();
  })
  .catch((err: unknown) => {
    globalThis.__formsError = String((err as { message?: string })?.message ?? err);
  });
