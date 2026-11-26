/**
 * AOT fixture for @angnative/web tests.
 *
 * This component is compiled by real Angular ngc into
 * `compiled/web-aot-fixture.component.js`. The compiled Ivy output is checked
 * in so `pnpm test` never needs the JIT compiler or generated dist artifacts.
 */
import { Component } from '@angular/core';
import {
  NaViewComponent,
  NaTextComponent,
} from '@angnative/common';

@Component({
  selector: 'app-web-test',
  standalone: true,
  imports: [NaViewComponent, NaTextComponent],
  template: `
    <na-view [padding]="'8px'">
      <na-text>{{ label }}</na-text>
      <div id="tap" (click)="onClick()">Tap</div>
    </na-view>
  `,
})
export class WebAotFixtureComponent {
  label = 'Hello';
  clicks = 0;

  onClick(): void {
    this.clicks++;
    this.label = `Clicked ${this.clicks}`;
  }
}
