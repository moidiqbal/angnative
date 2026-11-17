/**
 * counter.component.ts — demo counter component using native elements.
 *
 * Same Angular mental model as web: signals + templates + DI.
 * The only difference: tags are `na-*` instead of HTML tags, and rendering
 * goes through AngNative's native renderer (no WebView).
 *
 * Style bindings use individual property syntax ([style.width]="'200px'")
 * which maps to renderer.setStyle(key, value).
 */

import { Component, computed, inject } from '@angular/core';
import {
  NaViewComponent as NaView,
  NaTextComponent as NaText,
  NaButtonComponent as NaButton,
} from '@angnative/common';
import { counter, increment, decrement, reset, isEven } from './counter-signal';

@Component({
  selector: 'demo-counter',
  standalone: true,
  imports: [NaView, NaText, NaButton],
  template: `
    <na-view [style.flexDirection]="'column'" [style.padding]="'16px'">
      <na-text [style.fontSize]="'24px'" [style.fontWeight]="'bold'">Counter</na-text>
      <na-text [text]="counterDisplay()"></na-text>
      <na-view [style.flexDirection]="'row'" [style.gap]="'8px'" [style.marginTop]="'12px'">
        <na-button (press)="decrement()">−</na-button>
        <na-button (press)="reset()">reset</na-button>
        <na-button (press)="increment()">+</na-button>
      </na-view>
      <na-text [style.color]="isEven() ? 'green' : 'red'" [style.marginTop]="'8px'">
        {{ isEven() ? 'even' : 'odd' }}
      </na-text>
    </na-view>
  `,
})
export class CounterComponent {
  readonly counter = counter;
  readonly increment = increment;
  readonly decrement = decrement;
  readonly reset = reset;
  readonly isEven = isEven;

  readonly counterDisplay = computed(() => `Count: ${this.counter()}`);
}
