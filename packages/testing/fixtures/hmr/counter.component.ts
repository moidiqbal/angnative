/**
 * HMR proof fixture — an Angular component whose local signal state must
 * survive a real module replacement on both web and embedded QuickJS.
 */
import { CUSTOM_ELEMENTS_SCHEMA, Component, signal } from '@angular/core';

@Component({
  selector: 'na-hmr-counter',
  template: '<na-view><na-text>{{ count() }}</na-text></na-view>',
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class HmrCounterComponent {
  readonly count = signal(0);

  increment(): void {
    this.count.update((n) => n + 1);
  }

  snapshot(): { count: number } {
    return { count: this.count() };
  }

  restore(state: unknown): void {
    const count = (state as { count?: number } | undefined)?.count;
    if (typeof count === 'number') this.count.set(count);
  }
}

export function bootstrap(): HmrCounterComponent {
  return new HmrCounterComponent();
}
