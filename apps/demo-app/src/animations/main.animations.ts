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
    // Second trigger opts into the host-clock scheduler through a named easing.
    // It starts hidden and is only flipped by __setEasedAnimationState(), so the
    // historical deterministic-samples test is unaffected.
    trigger('easedFade', [
      state('hidden', style({ opacity: 0 })),
      state('visible', style({ opacity: 1 })),
      transition('hidden => visible', [
        animate(
          '100ms ease-in-out',
          keyframes([
            style({ opacity: 0, offset: 0 }),
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
      <div
        id="eased"
        [@easedFade]="easedState"
        (@easedFade.start)="onEasedStart($event)"
        (@easedFade.done)="onEasedDone($event)"
      >eased</div>
    </na-view>
  `,
})
export class AnimationsAppComponent {
  state = 'hidden';
  events: string[] = [];
  easedState = 'hidden';
  easedEvents: string[] = [];

  constructor(private readonly cdr: ChangeDetectorRef) {}

  onStart(_event: AnimationEvent): void {
    this.events.push('start');
  }

  onDone(_event: AnimationEvent): void {
    this.events.push('done');
  }

  onEasedStart(_event: AnimationEvent): void {
    this.easedEvents.push('start');
  }

  onEasedDone(_event: AnimationEvent): void {
    this.easedEvents.push('done');
  }

  setState(value: string): void {
    this.state = value;
    this.flush();
  }

  setEasedState(value: string): void {
    this.easedState = value;
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
  // eslint-disable-next-line no-var
  var __setEasedAnimationState: ((state: string) => void) | undefined;
  // eslint-disable-next-line no-var
  var __easedAnimationEvents: (() => string[]) | undefined;
  // eslint-disable-next-line no-var
  var __resetEasedAnimationEvents: (() => void) | undefined;
  // eslint-disable-next-line no-var
  var __flushEasedFrame: (() => void) | undefined;
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
    globalThis.__setEasedAnimationState = (value: string) => app.setEasedState(value);
    globalThis.__easedAnimationEvents = () => app.easedEvents.slice();
    globalThis.__resetEasedAnimationEvents = () => { app.easedEvents.length = 0; };
    globalThis.__flushEasedFrame = () => app.flush();
  })
  .catch((err: unknown) => {
    const error = err as { message?: string; stack?: string };
    globalThis.__animationsError = String(error.stack ?? error.message ?? err);
  });
