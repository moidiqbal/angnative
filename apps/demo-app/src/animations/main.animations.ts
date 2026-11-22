/**
 * main.animations.ts — AOT demo for Angular animations on the native renderer.
 *
 * The Rust acceptance test boots this bundle, flips `state` from `hidden` to
 * `visible`, and verifies that Angular's animation engine drives intermediate
 * `opacity` values through native `SetStyle` mutations.
 */
import {
  animate,
  keyframes,
  state,
  style,
  transition,
  trigger,
  type AnimationEvent,
} from '@angular/animations';
import {
  ChangeDetectorRef,
  Component,
  provideZonelessChangeDetection,
  type ApplicationRef,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import { provideNativeAngular, provideNativeAnimations } from '@angnative/platform';
import { NaViewComponent as NaView } from '@angnative/common';

@Component({
  selector: 'demo-animations-app',
  standalone: true,
  imports: [NaView],
  animations: [
    trigger('fade', [
      state('hidden', style({ opacity: 0 })),
      state('visible', style({ opacity: 1 })),
      transition('hidden => visible', [
        animate(
          '100ms',
          keyframes([
            style({ opacity: 0, offset: 0 }),
            style({ opacity: 0.5, offset: 0.5 }),
            style({ opacity: 1, offset: 1 }),
          ]),
        ),
      ]),
    ]),
  ],
  template: `
    <na-view>
      <div
        id="animated"
        [@fade]="state"
        (@fade.start)="onStart($event)"
        (@fade.done)="onDone($event)"
      >animated</div>
    </na-view>
  `,
})
export class AnimationsAppComponent {
  state = 'hidden';
  events: string[] = [];

  constructor(private readonly cdr: ChangeDetectorRef) {}

  onStart(_event: AnimationEvent): void {
    this.events.push('start');
  }

  onDone(_event: AnimationEvent): void {
    this.events.push('done');
  }

  setState(value: string): void {
    this.state = value;
    this.flush();
  }

  flush(): void {
    this.cdr.detectChanges();
  }
}

declare global {
  // eslint-disable-next-line no-var
  var __animationsError: string | undefined;
  // eslint-disable-next-line no-var
  var __setAnimationState: ((state: string) => void) | undefined;
  // eslint-disable-next-line no-var
  var __animationEvents: (() => string[]) | undefined;
  // eslint-disable-next-line no-var
  var __resetAnimationEvents: (() => void) | undefined;
}

bootstrapApplication(AnimationsAppComponent, {
  providers: [
    provideZonelessChangeDetection(),
    provideNativeAngular(),
    provideNativeAnimations(),
  ],
})
  .then((ref: ApplicationRef) => {
    const app = ref.components[0]!.instance as AnimationsAppComponent;
    globalThis.__setAnimationState = (value: string) => app.setState(value);
    globalThis.__animationEvents = () => app.events.slice();
    globalThis.__resetAnimationEvents = () => { app.events.length = 0; };
  })
  .catch((err: unknown) => {
    const error = err as { message?: string; stack?: string };
    globalThis.__animationsError = String(error.stack ?? error.message ?? err);
  });
