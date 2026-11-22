/**
 * app.component.ts — demo app root.
 */

import { Component } from '@angular/core';
import {
  NaViewComponent as NaView,
  NaTextComponent as NaText,
} from '@angnative/common';
import { CounterComponent } from './counter.component';

@Component({
  selector: 'demo-app',
  standalone: true,
  imports: [NaView, NaText, CounterComponent],
  template: `
    <na-view [style.flexDirection]="'column'" [style.padding]="'24px'" [style.gap]="'16px'">
      <na-text
        [style.width]="'220px'"
        [style.height]="'28px'"
        [style.fontSize]="'20px'"
        [style.fontWeight]="'bold'"
        [text]="'AngNative Demo'"
      ></na-text>
      <na-text
        [style.width]="'320px'"
        [style.height]="'20px'"
        [style.color]="'#666'"
        [text]="'Angular running natively — no WebView.'"
      ></na-text>
      <demo-counter></demo-counter>
    </na-view>
  `,
})
export class AppComponent {}
